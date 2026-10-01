//! Interactive key test mode for validating terminal input.
//!
//! This module provides a test mode that captures raw keyboard input
//! and displays both raw bytes and decoded key events.

use std::io::{self, Write};

/// Run interactive key test mode.
///
/// Press keys to see their raw bytes and decoded representation.
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
    writeln!(stdout, "Key Test Mode - Press keys to see their bytes and events")?;
    writeln!(stdout, "Press 'q' three times to exit")?;
    writeln!(stdout, "---")?;
    stdout.flush()?;

    let mut q_count = 0;

    // Read from stdin in raw mode
    use std::io::Read;
    let mut buffer = [0u8; 1024];
    loop {
        match io::stdin().read(&mut buffer) {
            Ok(n) if n > 0 => {
                let bytes = &buffer[..n];

                // Display raw bytes
                write!(stdout, "Bytes: ")?;
                for (i, &b) in bytes.iter().enumerate() {
                    if i > 0 {
                        write!(stdout, " ")?;
                    }
                    write!(stdout, "{:02x}", b)?;
                }
                write!(stdout, " | ASCII: ")?;
                for &b in bytes.iter() {
                    if b >= 32 && b < 127 {
                        write!(stdout, "{}", b as char)?;
                    } else {
                        write!(stdout, ".")?;
                    }
                }
                writeln!(stdout)?;
                stdout.flush()?;

                // Check for quit sequence
                if bytes.len() == 1 && bytes[0] == b'q' {
                    q_count += 1;
                    writeln!(stdout, "[q count: {}/3]", q_count)?;
                    if q_count >= 3 {
                        break;
                    }
                } else {
                    q_count = 0;
                }
            }
            Ok(0) => break,
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

    println!("Key Test Mode - Press keys to see their events");
    println!("Press 'q' three times to exit");
    println!("---");

    let mut q_count = 0;

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key_event) = event::read()? {
                println!("Event: {:?}", key_event);

                if let crossterm::event::KeyCode::Char('q') = key_event.code {
                    q_count += 1;
                    println!("[q count: {}/3]", q_count);
                    if q_count >= 3 {
                        break;
                    }
                } else {
                    q_count = 0;
                }
            }
        }
    }

    println!("Exited key test mode");
    Ok(())
}
