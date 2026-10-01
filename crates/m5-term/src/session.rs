//! A terminal session: raw mode and the alternate screen, plus the input modes that were
//! switched on (see [`crate::caps`]). Everything is undone when the [`Session`] is dropped
//! and when a panic starts, so a crash does not leave the terminal in raw mode, on the
//! alternate screen or with mouse reports and the like still switched on.

use std::io::{self, Write};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Switches to the alternate screen.
pub const ENTER_ALT_SCREEN: &[u8] = b"\x1b[?1049h";
/// Switches back to the main screen.
pub const LEAVE_ALT_SCREEN: &[u8] = b"\x1b[?1049l";

/// What has to be written to give the terminal back; shared with the panic hook.
#[derive(Debug, Default)]
struct Restore {
    /// Sequences that switch the input modes off.
    modes_off: Mutex<Vec<u8>>,
    /// Set once the terminal has been given back.
    done: AtomicBool,
}

impl Restore {
    /// Writes the mode-off sequences and the leave-alternate-screen sequence to `out`; only
    /// the first call writes anything. Returns whether this call was the first.
    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<bool> {
        if self.done.swap(true, Ordering::SeqCst) {
            return Ok(false);
        }
        let off = self.modes_off.lock().unwrap_or_else(|e| e.into_inner());
        out.write_all(&off)?;
        out.write_all(LEAVE_ALT_SCREEN)?;
        out.flush()?;
        Ok(true)
    }

    /// Gives the terminal back: the sequences, then cooked mode.
    fn run(&self) {
        let mut out = io::stdout();
        if matches!(self.write_to(&mut out), Ok(true)) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

/// Raw mode and the alternate screen for as long as the value lives.
#[derive(Debug)]
pub struct Session {
    restore: Arc<Restore>,
}

impl Session {
    /// Enters raw mode and the alternate screen and arranges for them to be left on a panic.
    ///
    /// The panic hook that is installed writes the sequences first and then calls the hook
    /// that was set before, so the panic message appears on the restored screen. The hook
    /// stays installed after the session ends; it does nothing then.
    pub fn enter() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let restore = Arc::new(Restore::default());
        let session = Session {
            restore: Arc::clone(&restore),
        };
        // From here a failure drops `session`, which undoes raw mode.
        let mut out = io::stdout();
        out.write_all(ENTER_ALT_SCREEN)?;
        out.flush()?;
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore.run();
            previous(info);
        }));
        Ok(session)
    }

    /// Sets what is written on the way out to switch the input modes off
    /// (`Modes::disable` of [`crate::caps::plan`]).
    pub fn set_modes_off(&self, bytes: &[u8]) {
        let mut off = self
            .restore
            .modes_off
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        off.clear();
        off.extend_from_slice(bytes);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.restore.run();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_writes_modes_off_then_leaves_the_alternate_screen() {
        let restore = Restore::default();
        *restore.modes_off.lock().unwrap() = b"\x1b[?1006l\x1b[<u".to_vec();
        let mut out = Vec::new();
        assert!(restore.write_to(&mut out).unwrap());
        assert_eq!(out, b"\x1b[?1006l\x1b[<u\x1b[?1049l");
    }

    #[test]
    fn restore_happens_once() {
        let restore = Restore::default();
        let mut first = Vec::new();
        let mut second = Vec::new();
        assert!(restore.write_to(&mut first).unwrap());
        assert!(!restore.write_to(&mut second).unwrap());
        assert_eq!(first, LEAVE_ALT_SCREEN);
        assert!(second.is_empty());
    }

    #[test]
    fn restore_after_a_poisoned_lock_still_writes() {
        let restore = Arc::new(Restore::default());
        let shared = Arc::clone(&restore);
        let result = std::thread::spawn(move || {
            let mut guard = shared.modes_off.lock().unwrap();
            guard.extend_from_slice(b"X");
            panic!("poison the lock");
        })
        .join();
        assert!(result.is_err());
        let mut out = Vec::new();
        assert!(restore.write_to(&mut out).unwrap());
        assert_eq!(out, b"X\x1b[?1049l");
    }
}
