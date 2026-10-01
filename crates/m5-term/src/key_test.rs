//! Interactive key test mode for validating terminal input.
//!
//! This module provides a test mode that captures raw keyboard input
//! and displays both raw bytes and decoded InputEvent structures using
//! Win32-compatible format (like far2l and f4).

use super::key::{InputEvent, vk_from_ascii};
use std::io::{self, Write};

/// Run interactive key test mode.
///
/// Press keys to see their raw bytes and decoded InputEvent representation.
/// Press 'q' three times in succession to exit.
#[cfg(unix)]
pub fn run_key_test() -> io::Result<()> {
    use crossterm::{
        execute,
        terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    // Enable raw mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Print header
    writeln!(stdout, "Key Test Mode (Win32 InputEvent Format)")?;
    writeln!(stdout, "Press keys to see raw bytes and decoded events")?;
    writeln!(stdout, "Press 'q' three times to exit")?;
    writeln!(stdout, "This validates that all sequences are passed through correctly")?;
    writeln!(stdout, "including APC far2l, kitty protocol, and ANSI sequences")?;
    writeln!(stdout, "---")?;
    stdout.flush()?;

    let mut q_count = 0;

    // Read from stdin in raw mode
    use std::io::Read;
    let mut buffer = [0u8; 1024];
    loop {
        match io::stdin().read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let bytes = &buffer[..n];

                // Display raw bytes in hex
                write!(stdout, "Raw: ")?;
                for (i, &b) in bytes.iter().enumerate() {
                    if i > 0 {
                        write!(stdout, " ")?;
                    }
                    write!(stdout, "{:02x}", b)?;
                }

                // Display ASCII representation
                write!(stdout, " | Chars: ")?;
                for &b in bytes.iter() {
                    if (32..127).contains(&b) {
                        write!(stdout, "{}", b as char)?;
                    } else if b == b'\x1b' {
                        write!(stdout, "ESC")?;
                    } else if b == b'\r' {
                        write!(stdout, "CR")?;
                    } else if b == b'\n' {
                        write!(stdout, "LF")?;
                    } else {
                        write!(stdout, "[{:02x}]", b)?;
                    }
                }
                writeln!(stdout)?;

                // Display as Win32-style InputEvent
                // For simple case: if it's a printable ASCII, create a key event
                if bytes.len() == 1 && (32..127).contains(&bytes[0]) {
                    let ch = bytes[0] as char;
                    let event = InputEvent::key(vk_from_ascii(ch), 0, ch, true)
                        .with_source("unix_raw".to_string());
                    writeln!(stdout, "Event: {}", event)?;
                } else {
                    // For sequences, just note the length
                    writeln!(stdout, "Sequence: {} bytes (raw pass-through)", bytes.len())?;
                }
                writeln!(stdout)?;
                stdout.flush()?;

                // Check for quit sequence
                if bytes.len() == 1 && bytes[0] == b'q' {
                    q_count += 1;
                    writeln!(stdout, "[quit count: {}/3]", q_count)?;
                    if q_count >= 3 {
                        break;
                    }
                } else {
                    q_count = 0;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                disable_raw_mode()?;
                execute!(stdout, LeaveAlternateScreen)?;
                return Err(e);
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    println!("Exited key test mode");
    Ok(())
}

#[cfg(not(unix))]
pub fn run_key_test() -> io::Result<()> {
    use crossterm::event;

    println!("Key Test Mode (Win32 InputEvent Format via crossterm)");
    println!("Press keys to see decoded events");
    println!("Press 'q' three times to exit");
    println!("---");

    let mut q_count = 0;

    loop {
        if event::poll(std::time::Duration::from_millis(100))?
            && let event::Event::Key(key_event) = event::read()?
        {
            // Convert crossterm event to our InputEvent format
            let event = match key_event.code {
                crossterm::event::KeyCode::Char(c) => {
                    let mut ev = InputEvent::key(vk_from_ascii(c), 0, c, true);
                    // Map crossterm modifiers to Win32 ControlKeyState
                    if key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                        ev.control_key_state = ev.control_key_state.with_shift();
                    }
                    if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        ev.control_key_state = ev.control_key_state.with_left_ctrl();
                    }
                    if key_event.modifiers.contains(crossterm::event::KeyModifiers::ALT) {
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
    }

    println!("Exited key test mode");
    Ok(())
}
