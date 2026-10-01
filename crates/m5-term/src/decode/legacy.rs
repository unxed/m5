//! Keys of xterm and VT-style terminals: CSI and SS3 sequences, with modifiers.
//!
//! Sources of the tables: the xterm control sequences document ("PC-Style Function
//! Keys", "VT220-style Function Keys", "Application keypad"), the Linux console
//! (`ESC [ [ A`..`E`) and rxvt (shifted and Ctrl arrows).

use super::csi::Csi;
use super::keys::{
    CTRL, ENHANCED, SHIFT, SRC_CSI, SRC_SS3, key_with_mods, legacy_event, nav_event, xterm_mods,
};
use crate::key::{InputEvent, vk};

/// Virtual key of the number in `CSI number ~`.
fn tilde_vk(code: u32) -> Option<u16> {
    let key = match code {
        1 | 7 => vk::HOME,
        2 => vk::INSERT,
        3 => vk::DELETE,
        4 | 8 => vk::END,
        5 => vk::PRIOR,
        6 => vk::NEXT,
        11..=15 => vk::F1 + (code - 11) as u16,
        17..=21 => vk::F1 + 5 + (code - 17) as u16,
        23..=26 => vk::F1 + 10 + (code - 23) as u16,
        28 | 29 => vk::F1 + 14 + (code - 28) as u16,
        31..=34 => vk::F1 + 16 + (code - 31) as u16,
        _ => return None,
    };
    Some(key)
}

/// Whether `CSI n ; m R` is a cursor position report rather than a modified F3.
fn is_cursor_report(csi: &Csi) -> bool {
    csi.params.len() >= 2 && csi.param(0) != Some(1)
}

/// Key event of a CSI sequence, or `None` if it is not a key (replies, unknown).
pub(crate) fn csi_key(csi: &Csi) -> Option<InputEvent> {
    let first = csi.param(0);
    let mods = xterm_mods(csi.param(1).unwrap_or(1));
    let final_byte = csi.final_byte;
    if final_byte == b'~' {
        let code = first?;
        if code == 27 {
            return other_keys(csi);
        }
        return Some(nav_event(tilde_vk(code)?, mods, SRC_CSI));
    }
    if final_byte == b'R' && is_cursor_report(csi) {
        return None;
    }
    if !matches!(first, None | Some(1)) {
        return None;
    }
    let (key, extra) = match final_byte {
        b'A' => (vk::UP, 0),
        b'B' => (vk::DOWN, 0),
        b'C' => (vk::RIGHT, 0),
        b'D' => (vk::LEFT, 0),
        b'H' => (vk::HOME, 0),
        b'F' => (vk::END, 0),
        b'E' => (vk::CLEAR, 0),
        b'P' => (vk::F1, 0),
        b'Q' => (vk::F1 + 1, 0),
        b'R' => (vk::F1 + 2, 0),
        b'S' => (vk::F1 + 3, 0),
        b'Z' => (vk::TAB, SHIFT),
        b'a' => (vk::UP, SHIFT),
        b'b' => (vk::DOWN, SHIFT),
        b'c' => (vk::RIGHT, SHIFT),
        b'd' => (vk::LEFT, SHIFT),
        _ => return None,
    };
    let mut ev = nav_event(key, mods | extra, SRC_CSI);
    if key == vk::TAB {
        ev.char_code = '\t';
    }
    Some(ev)
}

/// xterm `modifyOtherKeys`: `CSI 27 ; modifier ; code ~`.
fn other_keys(csi: &Csi) -> Option<InputEvent> {
    let mods = xterm_mods(csi.param(1).unwrap_or(1));
    let c = char::from_u32(csi.param(2)?)?;
    Some(key_with_mods(c, mods))
}

/// Key event of an SS3 sequence (`ESC O final`, or `ESC O digit final` with a modifier).
pub(crate) fn ss3_key(mods: u32, final_byte: u8) -> Option<InputEvent> {
    let keypad = |key: u16, ch: char, extra: u32| legacy_event(key, ch, mods | extra, SRC_SS3);
    let ev = match final_byte {
        b'A' => nav_event(vk::UP, mods, SRC_SS3),
        b'B' => nav_event(vk::DOWN, mods, SRC_SS3),
        b'C' => nav_event(vk::RIGHT, mods, SRC_SS3),
        b'D' => nav_event(vk::LEFT, mods, SRC_SS3),
        b'H' => nav_event(vk::HOME, mods, SRC_SS3),
        b'F' => nav_event(vk::END, mods, SRC_SS3),
        b'E' => nav_event(vk::CLEAR, mods, SRC_SS3),
        b'P'..=b'S' => nav_event(vk::F1 + u16::from(final_byte - b'P'), mods, SRC_SS3),
        b'a' => nav_event(vk::UP, mods | CTRL, SRC_SS3),
        b'b' => nav_event(vk::DOWN, mods | CTRL, SRC_SS3),
        b'c' => nav_event(vk::RIGHT, mods | CTRL, SRC_SS3),
        b'd' => nav_event(vk::LEFT, mods | CTRL, SRC_SS3),
        b'M' => keypad(vk::RETURN, '\r', ENHANCED),
        b'j' => keypad(vk::MULTIPLY, '*', 0),
        b'k' => keypad(vk::ADD, '+', 0),
        b'l' => keypad(vk::SEPARATOR, ',', 0),
        b'm' => keypad(vk::SUBTRACT, '-', 0),
        b'n' => keypad(vk::DECIMAL, '.', 0),
        b'o' => keypad(vk::DIVIDE, '/', ENHANCED),
        b'p'..=b'y' => {
            let digit = final_byte - b'p';
            keypad(vk::NUMPAD0 + u16::from(digit), char::from(b'0' + digit), 0)
        }
        _ => return None,
    };
    Some(ev)
}
