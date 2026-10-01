//! Decoders turning the byte stream of a terminal into [`InputEvent`]s.
//!
//! [`Decoder`] is a pure state machine: bytes go in through [`Decoder::feed`], events come
//! out, and no terminal or clock is involved. A sequence cut in the middle is kept until
//! more bytes arrive; the caller reports that nothing else came within its escape timeout
//! with [`Decoder::flush_timeout`], which is how a lone `Esc` key press is recognized.
//!
//! Supported so far (the kitty keyboard protocol is in [`kitty`]): plain characters (UTF-8) and control bytes, `Alt` as an `ESC` prefix,
//! and the keys of xterm-style terminals (CSI and SS3 sequences with modifiers).
//! Terminal replies that are not input (cursor position, device attributes, ...) and
//! string sequences (APC) are consumed and dropped.
//!
//! Events of protocols that have no key release (everything here) carry `key_down = true`
//! and `is_legacy = true`.

mod csi;
mod keys;
pub mod kitty;
mod legacy;

#[cfg(test)]
mod tests;

use crate::key::{InputEvent, vk};
use csi::{Csi, Scan, StrScan, scan, scan_string};
use keys::{ALT, SRC_CSI, Utf8, char_event, decode_char, key_with_mods, nav_event, xterm_mods};

const ESC: u8 = 0x1B;

/// Longest incomplete sequence kept waiting for its end, in bytes.
const MAX_PENDING: usize = 64 * 1024;

/// Incremental decoder of terminal input.
#[derive(Debug, Default)]
pub struct Decoder {
    /// Bytes of an incomplete sequence.
    buf: Vec<u8>,
}

/// Outcome of parsing at the start of the buffer.
enum Parse {
    /// This many bytes were consumed.
    Done(usize),
    /// The sequence is cut short; wait for more bytes.
    Incomplete,
}

impl Decoder {
    /// A decoder with no pending input.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the decoder holds the beginning of an unfinished sequence.
    pub fn has_pending(&self) -> bool {
        !self.buf.is_empty()
    }

    /// Decodes `bytes`, appending the events to `out`.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<InputEvent>) {
        self.buf.extend_from_slice(bytes);
        self.drain(false, out);
    }

    /// Resolves what is still pending after the escape timeout: a lone `ESC` becomes the
    /// `Esc` key, `ESC` and one character become `Alt` and that character, and an
    /// unfinished sequence is given up as an `Esc` key followed by its bytes as text.
    pub fn flush_timeout(&mut self, out: &mut Vec<InputEvent>) {
        self.drain(true, out);
    }

    fn drain(&mut self, flush: bool, out: &mut Vec<InputEvent>) {
        let mut pos = 0;
        while pos < self.buf.len() {
            let rest = &self.buf[pos..];
            let used = match parse(rest, out) {
                Parse::Done(n) => n,
                Parse::Incomplete if flush || rest.len() > MAX_PENDING => {
                    resolve_incomplete(rest, out)
                }
                Parse::Incomplete => break,
            };
            pos += used;
        }
        let remaining = self.buf.len() - pos;
        self.buf.copy_within(pos.., 0);
        self.buf.truncate(remaining);
    }
}

/// Parses one unit at the start of `buf` (not empty).
fn parse(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    if buf[0] == ESC {
        return parse_escape(buf, out);
    }
    match decode_char(buf) {
        Utf8::Char(c, n) => {
            out.push(char_event(c));
            Parse::Done(n)
        }
        Utf8::Invalid => {
            out.push(char_event('\u{FFFD}'));
            Parse::Done(1)
        }
        Utf8::Incomplete => Parse::Incomplete,
    }
}

/// Gives up waiting for the rest of an incomplete unit; returns the bytes consumed.
fn resolve_incomplete(rest: &[u8], out: &mut Vec<InputEvent>) -> usize {
    if rest[0] != ESC {
        // The end of the input cuts a UTF-8 sequence short.
        out.push(char_event('\u{FFFD}'));
        return rest.len();
    }
    if rest.len() == 2 && rest[1].is_ascii() && rest[1] != ESC {
        // `ESC [`, `ESC O` and `ESC _` alone are Alt+[, Alt+O and Alt+_.
        out.push(key_with_mods(char::from(rest[1]), ALT));
        return 2;
    }
    out.push(char_event('\u{1b}'));
    1
}

/// Parses a unit that starts with `ESC`.
fn parse_escape(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    if buf.len() < 2 {
        return Parse::Incomplete;
    }
    match buf[1] {
        b'[' => parse_csi(buf, out),
        b'O' => parse_ss3(buf, out),
        b'_' => parse_apc(buf),
        ESC => parse_double_escape(buf, out),
        _ => parse_alt_char(buf, out),
    }
}

/// `ESC` followed by a character: that character typed with Alt.
fn parse_alt_char(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    match decode_char(&buf[1..]) {
        Utf8::Char(c, n) => {
            out.push(key_with_mods(c, ALT));
            Parse::Done(1 + n)
        }
        Utf8::Incomplete => Parse::Incomplete,
        Utf8::Invalid => {
            out.push(char_event('\u{1b}'));
            Parse::Done(1)
        }
    }
}

/// `ESC ESC`: Alt+Esc, or Alt with the CSI/SS3 sequence that follows (some terminals
/// send the Alt prefix in front of the key's own sequence).
fn parse_double_escape(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    match buf.get(2) {
        None => Parse::Incomplete,
        Some(&(b'[' | b'O')) => {
            let start = out.len();
            match parse_escape(&buf[1..], out) {
                Parse::Done(n) => {
                    for ev in &mut out[start..] {
                        ev.control_key_state.0 |= ALT;
                    }
                    Parse::Done(n + 1)
                }
                Parse::Incomplete => Parse::Incomplete,
            }
        }
        Some(_) => {
            let mut ev = char_event('\u{1b}');
            ev.control_key_state.0 |= ALT;
            out.push(ev);
            Parse::Done(2)
        }
    }
}

/// `ESC O`: an SS3 sequence, `ESC O final` or `ESC O digit final` with a modifier.
fn parse_ss3(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    let Some(&third) = buf.get(2) else {
        return Parse::Incomplete;
    };
    let (mods, final_byte, len) = if third.is_ascii_digit() {
        let Some(&fourth) = buf.get(3) else {
            return Parse::Incomplete;
        };
        (xterm_mods(u32::from(third - b'0')), fourth, 4)
    } else {
        (0, third, 3)
    };
    match legacy::ss3_key(mods, final_byte) {
        Some(ev) => {
            out.push(ev);
            Parse::Done(len)
        }
        // Not a key: `ESC O` was Alt+O, and the next byte is a character of its own.
        None => parse_alt_char(buf, out),
    }
}

/// `ESC _`: an application program command. Consumed; no protocol uses it yet.
fn parse_apc(buf: &[u8]) -> Parse {
    match scan_string(buf) {
        StrScan::Incomplete => Parse::Incomplete,
        StrScan::Aborted(n) => Parse::Done(n),
        StrScan::Done(len) => Parse::Done(len),
    }
}

/// `ESC [`: a CSI sequence.
fn parse_csi(buf: &[u8], out: &mut Vec<InputEvent>) -> Parse {
    if buf.len() >= 3 && buf[2] == b'[' {
        // Linux console: `ESC [ [ A` .. `ESC [ [ E` are F1 .. F5.
        let Some(&code) = buf.get(3) else {
            return Parse::Incomplete;
        };
        if (b'A'..=b'E').contains(&code) {
            out.push(nav_event(vk::F1 + u16::from(code - b'A'), 0, SRC_CSI));
            return Parse::Done(4);
        }
    }
    match scan(buf) {
        Scan::Incomplete => Parse::Incomplete,
        Scan::Bad(n) => Parse::Done(n),
        Scan::Seq(seq, n) => {
            dispatch_csi(&seq, out);
            Parse::Done(n)
        }
    }
}

fn dispatch_csi(seq: &Csi, out: &mut Vec<InputEvent>) {
    if seq.private.is_some() || seq.has_intermediate {
        return;
    }
    if seq.final_byte == b'u' {
        kitty::decode_u(seq, out);
        return;
    }
    if let Some(ev) = legacy::csi_key(seq) {
        out.push(ev);
    }
}
