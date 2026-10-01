//! Helpers shared by the decoders: translation of characters and control bytes into
//! Win32-style key events, UTF-8 decoding and xterm modifier values.

use crate::key::{ControlKeyState, InputEvent, vk};

pub(crate) const SHIFT: u32 = ControlKeyState::SHIFT_PRESSED;
pub(crate) const ALT: u32 = ControlKeyState::LEFT_ALT_PRESSED;
pub(crate) const CTRL: u32 = ControlKeyState::LEFT_CTRL_PRESSED;
pub(crate) const ENHANCED: u32 = ControlKeyState::ENHANCED_KEY;

/// `input_source` of events built from plain characters and control bytes.
pub(crate) const SRC_CHAR: &str = "legacy_char";
/// `input_source` of events built from CSI sequences.
pub(crate) const SRC_CSI: &str = "legacy_csi";
/// `input_source` of events of the kitty keyboard protocol.
pub(crate) const SRC_KITTY: &str = "kitty";
/// `input_source` of events of win32-input-mode.
pub(crate) const SRC_WIN32: &str = "win32";
/// `input_source` of events built from SS3 sequences.
pub(crate) const SRC_SS3: &str = "legacy_ss3";

/// Builds a key-down event of a protocol without key releases.
pub(crate) fn legacy_event(vk_code: u16, ch: char, cks: u32, source: &str) -> InputEvent {
    let mut ev = InputEvent::key(vk_code, 0, ch, true);
    ev.control_key_state = ControlKeyState(cks);
    ev.is_legacy = true;
    ev.input_source = source.to_string();
    ev
}

/// Win32 "enhanced" keys: the navigation cluster to the left of the numeric keypad.
pub(crate) fn is_enhanced_vk(vk_code: u16) -> bool {
    let arrows = matches!(vk_code, vk::LEFT | vk::RIGHT | vk::UP | vk::DOWN);
    let pages = matches!(vk_code, vk::HOME | vk::END | vk::PRIOR | vk::NEXT);
    arrows || pages || matches!(vk_code, vk::INSERT | vk::DELETE)
}

/// Event of a key without a character (arrows, function keys, ...).
pub(crate) fn nav_event(vk_code: u16, mods: u32, source: &str) -> InputEvent {
    let cks = if is_enhanced_vk(vk_code) {
        mods | ENHANCED
    } else {
        mods
    };
    legacy_event(vk_code, '\0', cks, source)
}

/// Converts an xterm or kitty modifier parameter (1 + bit mask: Shift 1, Alt 2, Ctrl 4,
/// Super 8, Hyper 16, Meta 32, Caps Lock 64, Num Lock 128) to Win32 control key state bits.
/// Super, Hyper and Meta have no Win32 equivalent and are dropped.
pub(crate) fn xterm_mods(param: u32) -> u32 {
    let bits = param.saturating_sub(1);
    let mut cks = 0;
    if bits & 1 != 0 {
        cks |= SHIFT;
    }
    if bits & 2 != 0 {
        cks |= ALT;
    }
    if bits & 4 != 0 {
        cks |= CTRL;
    }
    if bits & 64 != 0 {
        cks |= ControlKeyState::CAPS_LOCK_ON;
    }
    if bits & 128 != 0 {
        cks |= ControlKeyState::NUM_LOCK_ON;
    }
    cks
}

/// Virtual key of a printable character on a US keyboard layout, and whether Shift
/// has to be held to type it. `None` for characters the layout table does not know.
pub(crate) fn vk_for_char(c: char) -> Option<(u16, bool)> {
    let pair = match c {
        'a'..='z' => (u16::from(c.to_ascii_uppercase() as u8), false),
        'A'..='Z' => (u16::from(c as u8), true),
        '0'..='9' => (u16::from(c as u8), false),
        ' ' => (vk::SPACE, false),
        '`' => (vk::OEM_3, false),
        '~' => (vk::OEM_3, true),
        '-' => (vk::OEM_MINUS, false),
        '_' => (vk::OEM_MINUS, true),
        '=' => (vk::OEM_PLUS, false),
        '+' => (vk::OEM_PLUS, true),
        '[' => (vk::OEM_4, false),
        '{' => (vk::OEM_4, true),
        ']' => (vk::OEM_6, false),
        '}' => (vk::OEM_6, true),
        '\\' => (vk::OEM_5, false),
        '|' => (vk::OEM_5, true),
        ';' => (vk::OEM_1, false),
        ':' => (vk::OEM_1, true),
        '\'' => (vk::OEM_7, false),
        '"' => (vk::OEM_7, true),
        ',' => (vk::OEM_COMMA, false),
        '<' => (vk::OEM_COMMA, true),
        '.' => (vk::OEM_PERIOD, false),
        '>' => (vk::OEM_PERIOD, true),
        '/' => (vk::OEM_2, false),
        '?' => (vk::OEM_2, true),
        ')' => (0x30, true),
        '!' => (0x31, true),
        '@' => (0x32, true),
        '#' => (0x33, true),
        '$' => (0x34, true),
        '%' => (0x35, true),
        '^' => (0x36, true),
        '&' => (0x37, true),
        '*' => (0x38, true),
        '(' => (0x39, true),
        _ => return None,
    };
    Some(pair)
}

/// Key event for one decoded character or control byte of a legacy terminal.
///
/// Control bytes become the key a Win32 console would report: Ctrl+letter with the
/// control character as `char_code`, Enter, Tab, Escape and Backspace as themselves.
pub(crate) fn char_event(c: char) -> InputEvent {
    let cp = c as u32;
    let (vk_code, ch, cks) = match c {
        '\0' => (vk::SPACE, ' ', CTRL),
        '\u{8}' | '\u{7f}' => (vk::BACK, '\u{8}', 0),
        '\t' => (vk::TAB, '\t', 0),
        '\r' => (vk::RETURN, '\r', 0),
        '\u{1b}' => (vk::ESCAPE, c, 0),
        '\u{1c}' => (vk::OEM_5, c, CTRL),
        '\u{1d}' => (vk::OEM_6, c, CTRL),
        '\u{1e}' => (0x36, c, CTRL),
        '\u{1f}' => (vk::OEM_MINUS, c, CTRL),
        '\u{1}'..='\u{1a}' => (vk::A + (cp as u16 - 1), c, CTRL),
        _ => match vk_for_char(c) {
            Some((code, true)) => (code, c, SHIFT),
            Some((code, false)) => (code, c, 0),
            None => (0, c, 0),
        },
    };
    legacy_event(vk_code, ch, cks, SRC_CHAR)
}

/// The control character produced by Ctrl and `c`, if there is one.
fn control_char(c: char) -> Option<char> {
    if c.is_ascii_alphabetic() || matches!(c, '[' | '\\' | ']' | '^' | '_') {
        Some(char::from(c as u8 & 0x1f))
    } else {
        None
    }
}

/// Key event for character `c` typed together with the modifiers `mods` (Win32 bits).
pub(crate) fn key_with_mods(c: char, mods: u32) -> InputEvent {
    let mut ev = char_event(c);
    let already_ctrl = ev.control_key_state.has_ctrl();
    ev.control_key_state = ControlKeyState(ev.control_key_state.0 | mods);
    if mods & SHIFT != 0 && c.is_ascii_lowercase() {
        ev.char_code = c.to_ascii_uppercase();
    }
    let ctrl_requested = mods & (CTRL | ControlKeyState::RIGHT_CTRL_PRESSED) != 0;
    let ctrl_char = control_char(c).filter(|_| ctrl_requested && !already_ctrl);
    if let Some(cc) = ctrl_char {
        ev.char_code = cc;
    }
    ev
}

/// Result of decoding one character from the start of a byte buffer.
pub(crate) enum Utf8 {
    /// A character and the number of bytes it takes.
    Char(char, usize),
    /// The first byte cannot start a valid sequence here; skip one byte.
    Invalid,
    /// The buffer ends in the middle of a sequence.
    Incomplete,
}

/// Decodes the first UTF-8 character of `buf`, which must not be empty.
pub(crate) fn decode_char(buf: &[u8]) -> Utf8 {
    let first = buf[0];
    let need = match first {
        0x00..=0x7F => return Utf8::Char(char::from(first), 1),
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return Utf8::Invalid,
    };
    if buf.len() < need {
        return if buf[1..].iter().all(|b| b & 0xC0 == 0x80) {
            Utf8::Incomplete
        } else {
            Utf8::Invalid
        };
    }
    let Ok(s) = std::str::from_utf8(&buf[..need]) else {
        return Utf8::Invalid;
    };
    s.chars()
        .next()
        .map_or(Utf8::Invalid, |c| Utf8::Char(c, need))
}
