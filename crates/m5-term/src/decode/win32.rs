//! The win32-input-mode of Windows Terminal (also spoken by ConPTY and far2l):
//! `CSI Vk ; Sc ; Uc ; Kd ; Cs ; Rc _`, the fields of a Win32 `KEY_EVENT_RECORD` written as
//! decimal numbers: virtual key code, virtual scan code, Unicode character (a UTF-16 code
//! unit), key down (1 or 0), control key state and repeat count.
//!
//! Omitted fields default to 0, except the repeat count, which defaults to 1. A character
//! outside the Basic Multilingual Plane arrives as two events, one per surrogate; the pair
//! is merged into a single event here.

use super::csi::Csi;
use super::keys::SRC_WIN32;
use crate::key::{ControlKeyState, EventType, InputEvent};

/// Turns the mode on (`DECSET 9001`).
pub const ENABLE: &[u8] = b"\x1b[?9001h";
/// Turns the mode off.
pub const DISABLE: &[u8] = b"\x1b[?9001l";

/// Decodes `CSI ... _`. `high` holds the first half of a surrogate pair between calls.
pub(crate) fn decode(csi: &Csi, out: &mut Vec<InputEvent>, high: &mut Option<u16>) {
    let field = |i: usize| csi.param(i).unwrap_or(0);
    let unit = field(2).min(0xFFFF) as u16;
    let ch = match unit {
        0xD800..=0xDBFF => {
            *high = Some(unit);
            return;
        }
        0xDC00..=0xDFFF => {
            let Some(hi) = high.take() else {
                return;
            };
            let pair = [hi, unit];
            match char::decode_utf16(pair).next() {
                Some(Ok(c)) => c,
                _ => return,
            }
        }
        _ => {
            // A lone high surrogate followed by something else is dropped.
            *high = None;
            char::from_u32(u32::from(unit)).unwrap_or('\0')
        }
    };
    let repeat = match csi.param(5) {
        Some(0) | None => 1,
        Some(n) => n.min(0xFFFF) as u16,
    };
    let mut ev = InputEvent::key(
        field(0).min(0xFFFF) as u16,
        field(1).min(0xFFFF) as u16,
        ch,
        field(3) == 1,
    );
    ev.repeat_count = repeat;
    ev.control_key_state = ControlKeyState(field(4));
    ev.input_source = SRC_WIN32.to_string();
    out.push(ev);
}

/// Writes a key event as win32-input-mode sequences (two for a character outside the
/// Basic Multilingual Plane). `None` if the event is not a key event.
pub fn encode(ev: &InputEvent) -> Option<Vec<u8>> {
    if ev.event_type != EventType::Key {
        return None;
    }
    let mut units = [0u16; 2];
    let units: &[u16] = ev.char_code.encode_utf16(&mut units);
    let repeat = ev.repeat_count.max(1);
    let mut text = String::new();
    for unit in units {
        text.push_str(&format!(
            "\x1b[{};{};{};{};{};{}_",
            ev.virtual_key_code,
            ev.virtual_scan_code,
            unit,
            u8::from(ev.key_down),
            ev.control_key_state.0,
            repeat
        ));
    }
    Some(text.into_bytes())
}
