//! Bracketed paste (xterm mode 2004): the terminal wraps pasted text in `CSI 200 ~` and
//! `CSI 201 ~` so the application can tell it from typing.
//!
//! The decoder reports the two markers as [`InputEvent::paste`] events (`paste_start` true and
//! false). Between them every byte is text: escape sequences are not interpreted and control
//! bytes are not keys. The text arrives as key-down events, one per character, with the
//! virtual key of the character when it has one (see D-16 in `docs/DECISIONS.md`). A paste
//! that is never closed keeps the decoder in paste mode.

use super::keys::{SRC_CHAR, char_event, legacy_event};
use crate::key::InputEvent;

/// Turns bracketed paste on.
pub const ENABLE: &[u8] = b"\x1b[?2004h";
/// Turns bracketed paste off.
pub const DISABLE: &[u8] = b"\x1b[?2004l";

/// The sequence that ends a paste.
pub(crate) const END: &[u8] = b"\x1b[201~";

const SRC_PASTE: &str = "bracketed_paste";

/// Marker event at the start (`true`) or end (`false`) of a paste.
pub(crate) fn marker(start: bool) -> InputEvent {
    let mut ev = InputEvent::paste(start);
    ev.input_source = SRC_PASTE.to_string();
    ev
}

/// Key event of one character of pasted text.
pub(crate) fn text_event(c: char) -> InputEvent {
    let mut ev = match c {
        '\r' | '\n' => char_event('\r'),
        '\t' => char_event('\t'),
        _ if c.is_control() => legacy_event(0, c, 0, SRC_CHAR),
        _ => char_event(c),
    };
    if c == '\n' {
        ev.char_code = '\n';
    }
    ev.input_source = SRC_PASTE.to_string();
    ev
}
