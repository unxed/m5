//! Interactive key test mode for validating terminal input.
//!
//! This module provides a test mode that captures raw keyboard input
//! and displays both raw bytes and decoded InputEvent structures using
//! Win32-compatible format (like far2l and f4).

use super::key::InputEvent;
use std::io;
#[cfg(unix)]
use std::io::Write;

/// Counts consecutive presses of a plain `q`; three of them end the key test.
///
/// It works on decoded events, not on bytes, so it does not depend on the keyboard mode of
/// the terminal: `q` is a single byte in legacy mode and `CSI 113;;113 u` in kitty mode.
/// Releases and repeats of other keys do not break the series, presses of other keys do.
#[cfg(unix)]
#[derive(Debug, Default)]
struct QuitCounter {
    count: u8,
}

#[cfg(unix)]
impl QuitCounter {
    const NEEDED: u8 = 3;

    /// Takes one event; returns the series length after it when it was a press of `q`.
    fn feed(&mut self, ev: &InputEvent) -> Option<u8> {
        use super::key::EventType;
        if ev.event_type != EventType::Key || !ev.key_down {
            return None;
        }
        let state = ev.control_key_state;
        let plain_q = ev.char_code == 'q' && !state.has_ctrl() && !state.has_alt();
        if plain_q {
            self.count += 1;
            Some(self.count)
        } else {
            self.count = 0;
            None
        }
    }

    fn done(&self) -> bool {
        self.count >= Self::NEEDED
    }
}

/// Writes `text` and ends the line the way a raw-mode terminal needs it: with CR LF
/// (output post-processing is off, a lone LF would only move down and leave the column).
#[cfg(unix)]
fn line(out: &mut impl Write, text: &str) -> io::Result<()> {
    write!(out, "{text}\r\n")
}

/// Raw mode and the alternate screen; both are undone when the guard is dropped, so an
/// error or a panic does not leave the terminal broken.
#[cfg(unix)]
struct RawScreen;

#[cfg(unix)]
impl RawScreen {
    fn enter() -> io::Result<Self> {
        use crossterm::execute;
        use crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};
        enable_raw_mode()?;
        let guard = RawScreen;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(guard)
    }
}

#[cfg(unix)]
impl Drop for RawScreen {
    fn drop(&mut self) {
        use crossterm::{
            execute,
            terminal::{LeaveAlternateScreen, disable_raw_mode},
        };
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Hex dump and character view of one block of input bytes.
#[cfg(unix)]
fn describe_raw(bytes: &[u8]) -> String {
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let mut chars = String::new();
    for &b in bytes {
        match b {
            32..=126 => chars.push(b as char),
            0x1b => chars.push_str("ESC"),
            b'\r' => chars.push_str("CR"),
            b'\n' => chars.push_str("LF"),
            _ => chars.push_str(&format!("[{b:02x}]")),
        }
    }
    format!("Raw: {} | Chars: {chars}", hex.join(" "))
}

/// Run interactive key test mode.
///
/// Press keys to see their raw bytes and decoded InputEvent representation.
/// Press 'q' three times in succession to exit.
#[cfg(unix)]
pub fn run_key_test() -> io::Result<()> {
    use super::Decoder;
    use std::io::Read;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Duration;

    /// How long a lone `ESC` waits for the rest of a sequence.
    const ESC_TIMEOUT: Duration = Duration::from_millis(50);

    // The reader thread lets the loop below notice a timeout between reads.
    let (tx, rx) = mpsc::channel::<io::Result<Vec<u8>>>();
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut buffer = [0u8; 1024];
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(Ok(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });

    let screen = RawScreen::enter()?;
    let mut stdout = io::stdout();

    line(&mut stdout, "Key Test Mode (Win32 InputEvent Format)")?;
    line(&mut stdout, "Press keys to see raw bytes and decoded events")?;
    line(&mut stdout, "Press 'q' three times to exit")?;
    line(
        &mut stdout,
        "This validates that all sequences are passed through correctly",
    )?;
    line(
        &mut stdout,
        "including APC far2l, kitty protocol, and ANSI sequences",
    )?;
    line(&mut stdout, "---")?;
    stdout.flush()?;

    let mut decoder = Decoder::new();
    let mut quit = QuitCounter::default();
    let mut events = Vec::new();
    let mut result = Ok(());

    while !quit.done() {
        events.clear();
        match rx.recv_timeout(ESC_TIMEOUT) {
            Ok(Ok(bytes)) => {
                if let Err(e) = line(&mut stdout, &describe_raw(&bytes)) {
                    result = Err(e);
                    break;
                }
                decoder.feed(&bytes, &mut events);
            }
            Ok(Err(e)) => {
                result = Err(e);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                if decoder.has_pending() {
                    decoder.flush_timeout(&mut events);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
        for ev in &events {
            if let Some(n) = quit.feed(ev) {
                let shown = format!("[quit count: {n}/{}]", QuitCounter::NEEDED);
                if let Err(e) = line(&mut stdout, &shown) {
                    result = Err(e);
                }
            }
        }
        if let Err(e) = stdout.flush() {
            result = Err(e);
        }
        if result.is_err() {
            break;
        }
    }

    drop(screen);
    result?;
    println!("Exited key test mode");
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::Decoder;

    /// Feeds the byte blocks to a decoder and the events to a counter; returns the counter.
    fn run(blocks: &[&[u8]]) -> QuitCounter {
        let mut decoder = Decoder::new();
        let mut counter = QuitCounter::default();
        for block in blocks {
            let mut events = Vec::new();
            decoder.feed(block, &mut events);
            for ev in &events {
                counter.feed(ev);
            }
        }
        counter
    }

    #[test]
    fn three_plain_q_bytes_quit() {
        assert!(run(&[b"q", b"q", b"q"]).done());
        assert!(!run(&[b"q", b"q"]).done());
    }

    #[test]
    fn three_q_in_kitty_mode_quit() {
        // Press and release of `q` with the "all keys as escape codes" flags.
        let press: &[u8] = b"\x1b[113;;113u";
        let release: &[u8] = b"\x1b[113;1:3u";
        assert!(run(&[press, release, press, release, press, release]).done());
        assert!(!run(&[press, release, press, release]).done());
    }

    #[test]
    fn plain_q_in_kitty_mode_without_text() {
        assert!(run(&[b"\x1b[113u", b"\x1b[113u", b"\x1b[113u"]).done());
    }

    #[test]
    fn another_key_resets_the_series() {
        assert!(!run(&[b"q", b"q", b"a", b"q"]).done());
        assert!(run(&[b"q", b"q", b"a", b"q", b"q", b"q"]).done());
    }

    #[test]
    fn release_of_another_key_keeps_the_series() {
        let release_a: &[u8] = b"\x1b[97;1:3u";
        assert!(run(&[b"\x1b[113u", release_a, b"\x1b[113u", b"\x1b[113u"]).done());
    }

    #[test]
    fn ctrl_q_and_alt_q_are_not_q() {
        assert!(!run(&[b"\x11", b"\x11", b"\x11"]).done());
        assert!(!run(&[b"\x1bq", b"\x1bq", b"\x1bq"]).done());
        assert!(!run(&[b"\x1b[113;5u", b"\x1b[113;5u", b"\x1b[113;5u"]).done());
    }

    #[test]
    fn lines_end_with_cr_lf() {
        let mut out = Vec::new();
        line(&mut out, "abc").unwrap();
        assert_eq!(out, b"abc\r\n");
    }

    #[test]
    fn raw_dump_shows_bytes_and_names() {
        assert_eq!(describe_raw(b"\x1b[A"), "Raw: 1b 5b 41 | Chars: ESC[A");
        assert_eq!(describe_raw(b"\r\x01"), "Raw: 0d 01 | Chars: CR[01]");
    }
}

#[cfg(not(unix))]
pub fn run_key_test() -> io::Result<()> {
    use super::key::vk_from_ascii;
    use crossterm::event;

    println!("Key Test Mode (Win32 InputEvent Format via crossterm)");
    println!("Press keys to see decoded events");
    println!("Press 'q' three times to exit");
    println!("---");

    let mut q_count = 0;

    loop {
        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        let event::Event::Key(key_event) = event::read()? else {
            continue;
        };
        // Convert crossterm event to our InputEvent format
        let event = match key_event.code {
            crossterm::event::KeyCode::Char(c) => {
                let mut ev = InputEvent::key(vk_from_ascii(c), 0, c, true);
                // Map crossterm modifiers to Win32 ControlKeyState
                if key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SHIFT)
                {
                    ev.control_key_state = ev.control_key_state.with_shift();
                }
                if key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    ev.control_key_state = ev.control_key_state.with_left_ctrl();
                }
                if key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::ALT)
                {
                    ev.control_key_state = ev.control_key_state.with_left_alt();
                }
                ev.with_source("crossterm".to_string())
            }
            _ => {
                let mut ev = InputEvent::key(0, 0, '\0', true);
                ev.input_source = "crossterm_special".to_string();
                ev
            }
        };

        println!("Event: {}", event);

        if let crossterm::event::KeyCode::Char('q') = key_event.code {
            q_count += 1;
            println!("[quit count: {}/3]", q_count);
            if q_count >= 3 {
                break;
            }
        } else {
            q_count = 0;
        }
    }

    println!("Exited key test mode");
    Ok(())
}
