//! Negotiation of the terminal's abilities at start (DESIGN 5.1.2) and the input modes to
//! switch on (and off again on exit) as a result.
//!
//! The program sends one batch: the far2l query, the kitty keyboard query `CSI ? u` and
//! the primary device attributes request `CSI c`. A terminal answers the requests it knows,
//! in order, and every terminal answers `CSI c`, so that answer is the end marker: when it
//! has arrived nothing more is coming. [`Probe`] reads the answers out of the byte stream;
//! [`plan`] turns them into the sequences that switch modes on and off; [`negotiate`] runs
//! the whole exchange over a writer and a reader function, so it needs no terminal to be tested.
//!
//! Choice of the keyboard mode: an answer from far2l means far2l mode (kitty and
//! win32-input-mode stay off); otherwise a kitty answer `CSI ? flags u` enables the
//! "disambiguate" flag; otherwise the legacy keys are used. win32-input-mode is only
//! switched on when [`Options::win32`] asks for it (it is off by default, Q-05). Bracketed
//! paste, focus reports and SGR mouse reports are switched on in every case when asked for.

use crate::decode::csi::{Csi, Scan, StrScan, scan, scan_string};
use crate::decode::{far2l, focus, kitty, mouse, paste, win32};
use std::io::{self, Write};
use std::time::{Duration, Instant};

const ESC: u8 = 0x1B;

/// Primary device attributes request; every terminal answers it, so it ends the probe.
pub const PRIMARY_DA: &[u8] = b"\x1b[c";

/// What the terminal said about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    /// The terminal answered the far2l query with `ESC _ far2lok`.
    pub far2l: bool,
    /// The terminal answered `CSI ? flags u`: the flags currently active (kitty keyboard protocol).
    pub kitty_flags: Option<u8>,
    /// The answer to `CSI c` arrived, so the terminal has said everything it was going to say.
    pub primary_da: bool,
}

/// Reader of the answers to the probe batch.
///
/// Bytes that are not answers (keys typed while the probe was running, answers to other
/// requests) are kept apart and can be taken with [`Probe::take_input`] to be fed to the
/// input decoder.
#[derive(Debug, Default)]
pub struct Probe {
    /// Bytes of an incomplete sequence.
    buf: Vec<u8>,
    caps: Capabilities,
    input: Vec<u8>,
    done: bool,
}

impl Probe {
    /// A probe that has seen nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the probe is over: the `CSI c` answer arrived or [`Probe::timeout`] was called.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// What has been learned so far.
    pub fn capabilities(&self) -> Capabilities {
        self.caps
    }

    /// Takes the bytes that were not answers to the probe, in order.
    pub fn take_input(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.input)
    }

    /// Reads `bytes` from the terminal. After the end marker everything is plain input.
    pub fn feed(&mut self, bytes: &[u8]) {
        if self.done {
            self.input.extend_from_slice(bytes);
            return;
        }
        self.buf.extend_from_slice(bytes);
        self.drain(false);
    }

    /// Ends the probe because the time is up: an unfinished sequence becomes plain input.
    pub fn timeout(&mut self) {
        if !self.done {
            self.drain(true);
            self.done = true;
        }
    }

    fn drain(&mut self, flush: bool) {
        let buf = std::mem::take(&mut self.buf);
        let mut pos = 0;
        while pos < buf.len() && !self.done {
            match self.take_unit(&buf[pos..], flush) {
                Some(used) => pos += used,
                None => break,
            }
        }
        if self.done {
            self.input.extend_from_slice(&buf[pos..]);
        } else {
            self.buf = buf[pos..].to_vec();
        }
    }

    /// Handles the unit at the start of `rest`; `None` if it is cut short and `flush` is off.
    fn take_unit(&mut self, rest: &[u8], flush: bool) -> Option<usize> {
        if rest[0] != ESC {
            self.input.push(rest[0]);
            return Some(1);
        }
        match rest.get(1) {
            None => self.give_up(rest, 1, flush),
            Some(b'[') => match scan(rest) {
                Scan::Incomplete => self.give_up(rest, rest.len(), flush),
                Scan::Bad(n) => {
                    self.input.extend_from_slice(&rest[..n]);
                    Some(n)
                }
                Scan::Seq(seq, n) => {
                    if !self.answer(&seq) {
                        self.input.extend_from_slice(&rest[..n]);
                    }
                    Some(n)
                }
            },
            Some(b'_') => match scan_string(rest) {
                StrScan::Incomplete => self.give_up(rest, rest.len(), flush),
                StrScan::Aborted(n) => {
                    self.input.extend_from_slice(&rest[..n]);
                    Some(n)
                }
                StrScan::Done { end, len } => {
                    if rest[2..end] == *b"far2lok" {
                        self.caps.far2l = true;
                    } else {
                        // Not the answer: the far2l events that follow it are input.
                        self.input.extend_from_slice(&rest[..len]);
                    }
                    Some(len)
                }
            },
            Some(_) => {
                self.input.push(ESC);
                Some(1)
            }
        }
    }

    fn give_up(&mut self, rest: &[u8], len: usize, flush: bool) -> Option<usize> {
        if flush {
            self.input.extend_from_slice(&rest[..len]);
            Some(len)
        } else {
            None
        }
    }

    /// Takes the sequence if it is an answer to the probe.
    fn answer(&mut self, seq: &Csi) -> bool {
        if seq.private != Some(b'?') || seq.has_intermediate {
            return false;
        }
        match seq.final_byte {
            b'u' => {
                let flags = seq.param(0).unwrap_or(0);
                self.caps.kitty_flags = Some(u8::try_from(flags).unwrap_or(u8::MAX));
                true
            }
            b'c' => {
                self.caps.primary_da = true;
                self.done = true;
                true
            }
            _ => false,
        }
    }
}

/// What the program wants switched on, as far as the terminal allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Ask whether the terminal speaks the far2l extensions (see [`far2l_allowed`]).
    pub far2l: bool,
    /// Switch win32-input-mode on (when far2l is not in use).
    pub win32: bool,
    /// SGR mouse reports (buttons and drag).
    pub mouse: bool,
    /// Focus reports.
    pub focus: bool,
    /// Bracketed paste.
    pub paste: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            far2l: true,
            win32: false,
            mouse: true,
            focus: true,
            paste: true,
        }
    }
}

/// Whether the far2l query may be sent: not under `screen` or `tmux` (they would swallow or
/// mangle it) and not when `M5_FAR2L` is `0`. Arguments are the values of `TERM` and `M5_FAR2L`.
pub fn far2l_allowed(term: Option<&str>, m5_far2l: Option<&str>) -> bool {
    if m5_far2l == Some("0") {
        return false;
    }
    !term.is_some_and(|t| t.starts_with("screen") || t.starts_with("tmux"))
}

/// The probe batch: the far2l query (if asked for), the kitty query and the primary DA request.
pub fn probe_request(opts: &Options) -> Vec<u8> {
    let mut out = Vec::new();
    if opts.far2l {
        out.extend_from_slice(far2l::QUERY);
    }
    out.extend_from_slice(kitty::QUERY);
    out.extend_from_slice(PRIMARY_DA);
    out
}

/// Which keyboard protocol was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardMode {
    /// The far2l extensions (already switched on by the query).
    Far2l,
    /// The kitty keyboard protocol with the "disambiguate" flag.
    Kitty,
    /// win32-input-mode.
    Win32,
    /// Plain xterm sequences.
    Legacy,
}

/// The sequences that switch the chosen modes on and off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modes {
    pub keyboard: KeyboardMode,
    /// To be written after the probe.
    pub enable: Vec<u8>,
    /// To be written on exit, on suspend and on a panic: the reverse of `enable`.
    pub disable: Vec<u8>,
}

/// Chooses the modes for the answers in `caps`.
pub fn plan(caps: &Capabilities, opts: &Options) -> Modes {
    let mut enable = Vec::new();
    let mut off: Vec<&[u8]> = Vec::new();
    let keyboard = if caps.far2l {
        // Switched on by the query; only the way back is needed.
        off.push(far2l::DISABLE);
        KeyboardMode::Far2l
    } else if caps.kitty_flags.is_some() {
        enable.extend_from_slice(&kitty::enable(kitty::DISAMBIGUATE));
        off.push(kitty::POP);
        KeyboardMode::Kitty
    } else if opts.win32 {
        enable.extend_from_slice(win32::ENABLE);
        off.push(win32::DISABLE);
        KeyboardMode::Win32
    } else {
        KeyboardMode::Legacy
    };
    if opts.paste {
        enable.extend_from_slice(paste::ENABLE);
        off.push(paste::DISABLE);
    }
    if opts.focus {
        enable.extend_from_slice(focus::ENABLE);
        off.push(focus::DISABLE);
    }
    if opts.mouse {
        enable.extend_from_slice(mouse::ENABLE);
        off.push(mouse::DISABLE);
    }
    let disable = off.iter().rev().flat_map(|s| s.iter().copied()).collect();
    Modes {
        keyboard,
        enable,
        disable,
    }
}

/// A function that waits up to the given time for input and returns what arrived, if anything.
pub type Reader = dyn FnMut(Duration) -> io::Result<Option<Vec<u8>>>;

/// Result of [`negotiate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negotiated {
    pub caps: Capabilities,
    pub modes: Modes,
    /// Bytes read during the probe that were not answers (typed keys); feed them to the decoder.
    pub input: Vec<u8>,
}

/// Runs the negotiation: writes the probe batch to `out`, reads answers with `read` until the
/// answer to `CSI c` arrives or `timeout` is over, writes the sequences of the chosen modes
/// and returns what was found.
///
/// `read(wait)` returns the bytes that arrived within `wait`, or `None` if nothing did.
pub fn negotiate<W: Write>(
    out: &mut W,
    read: &mut Reader,
    opts: &Options,
    timeout: Duration,
) -> io::Result<Negotiated> {
    out.write_all(&probe_request(opts))?;
    out.flush()?;
    let deadline = Instant::now() + timeout;
    let mut probe = Probe::new();
    while !probe.is_done() {
        let wait = deadline.saturating_duration_since(Instant::now());
        let bytes = if wait.is_zero() { None } else { read(wait)? };
        match bytes {
            Some(bytes) => probe.feed(&bytes),
            None => probe.timeout(),
        }
    }
    let caps = probe.capabilities();
    let modes = plan(&caps, opts);
    out.write_all(&modes.enable)?;
    out.flush()?;
    Ok(Negotiated {
        caps,
        modes,
        input: probe.take_input(),
    })
}

#[cfg(test)]
mod tests;
