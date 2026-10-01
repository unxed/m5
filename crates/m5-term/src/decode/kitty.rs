//! The kitty keyboard protocol: decoding of its key events into [`InputEvent`]s and the
//! reverse conversion of an [`InputEvent`] into kitty escape sequences.
//!
//! A key event is `CSI key[:shifted[:base]] ; modifiers[:event] ; text u`, or the older
//! `CSI 1 ; modifiers[:event] letter` / `CSI number ; modifiers[:event] ~` forms for the
//! keys that had them. `modifiers` is 1 + a bit mask (Shift 1, Alt 2, Ctrl 4, Super 8,
//! Hyper 16, Meta 32, Caps Lock 64, Num Lock 128); `event` is 1 press, 2 repeat, 3 release.
//! The specification is <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>.
//!
//! Conventions of the conversion (see D-11 and D-12 in `docs/DECISIONS.md`):
//! - Super, Hyper and Meta have no Win32 control key state bit and are dropped.
//! - A repeat event is a key press (`repeat_count` stays 1); the encoder never writes
//!   repeat events.
//! - Keypad keys that kitty reports as navigation keys (`KP_LEFT`, ...) become the plain
//!   navigation virtual key without `ENHANCED_KEY`, as Windows reports the keypad with
//!   Num Lock off; the encoder maps them back. Keys of the cluster next to the keypad
//!   carry `ENHANCED_KEY`.
//! - Media keys, `F25`..`F35`, Hyper and Meta keys have no virtual key here; their events
//!   are dropped.

use super::csi::Csi;
use super::keys::{
    ALT, CTRL, ENHANCED, SHIFT, SRC_KITTY, char_event, key_with_mods, vk_for_char, xterm_mods,
};
use crate::key::{ControlKeyState, EventType, InputEvent, vk};

/// Flag of the progressive enhancement: report ambiguous keys as escape codes.
pub const DISAMBIGUATE: u8 = 0b1;
/// Flag: report press, repeat and release events.
pub const EVENT_TYPES: u8 = 0b10;
/// Flag: report the shifted and base layout keys.
pub const ALTERNATE_KEYS: u8 = 0b100;
/// Flag: report all keys, including text keys and modifiers, as escape codes.
pub const ALL_KEYS: u8 = 0b1000;
/// Flag: report the text a key produces (only together with [`ALL_KEYS`]).
pub const ASSOCIATED_TEXT: u8 = 0b10000;

/// Query of the current flags; the terminal answers `CSI ? flags u`.
pub const QUERY: &[u8] = b"\x1b[?u";
/// Pops the flags pushed by [`enable`].
pub const POP: &[u8] = b"\x1b[<u";

/// Sequence that pushes `flags` onto the terminal's stack and so enables them.
pub fn enable(flags: u8) -> Vec<u8> {
    format!("\x1b[>{flags}u").into_bytes()
}

const RIGHT_CTRL: u32 = ControlKeyState::RIGHT_CTRL_PRESSED;
const RIGHT_ALT: u32 = ControlKeyState::RIGHT_ALT_PRESSED;
const CAPS: u32 = ControlKeyState::CAPS_LOCK_ON;
const NUM: u32 = ControlKeyState::NUM_LOCK_ON;

/// Scan codes of the Shift keys (the right one is `RIGHT_SHIFT_VSC` of far2l).
const LEFT_SHIFT_SCAN: u16 = 0x2A;
const RIGHT_SHIFT_SCAN: u16 = 0x36;

// Code points of the functional keys that are not in the legacy forms.
const CAPS_LOCK: u32 = 57358;
const SCROLL_LOCK: u32 = 57359;
const NUM_LOCK: u32 = 57360;
const PRINT_SCREEN: u32 = 57361;
const PAUSE: u32 = 57362;
const MENU: u32 = 57363;
const F13: u32 = 57376;
const KP_0: u32 = 57399;
const KP_DECIMAL: u32 = 57409;
const KP_DIVIDE: u32 = 57410;
const KP_MULTIPLY: u32 = 57411;
const KP_SUBTRACT: u32 = 57412;
const KP_ADD: u32 = 57413;
const KP_ENTER: u32 = 57414;
const KP_EQUAL: u32 = 57415;
const KP_SEPARATOR: u32 = 57416;
const KP_LEFT: u32 = 57417;
const KP_RIGHT: u32 = 57418;
const KP_UP: u32 = 57419;
const KP_DOWN: u32 = 57420;
const KP_PAGE_UP: u32 = 57421;
const KP_PAGE_DOWN: u32 = 57422;
const KP_HOME: u32 = 57423;
const KP_END: u32 = 57424;
const KP_INSERT: u32 = 57425;
const KP_DELETE: u32 = 57426;
const KP_BEGIN: u32 = 57427;
const LEFT_SHIFT: u32 = 57441;
const LEFT_CONTROL: u32 = 57442;
const LEFT_ALT: u32 = 57443;
const LEFT_SUPER: u32 = 57444;
const RIGHT_SHIFT: u32 = 57447;
const RIGHT_CONTROL: u32 = 57448;
const RIGHT_ALT_KEY: u32 = 57449;
const RIGHT_SUPER: u32 = 57450;
const ISO_LEVEL3_SHIFT: u32 = 57453;

/// First and last code point of the private use area kitty takes functional keys from.
const FUNCTIONAL_RANGE: std::ops::RangeInclusive<u32> = 57344..=63743;

/// Virtual key, character and extra control key state bits of a functional key number.
fn functional(code: u32) -> Option<(u16, char, u32)> {
    let key = match code {
        27 => (vk::ESCAPE, '\u{1b}', 0),
        13 => (vk::RETURN, '\r', 0),
        9 => (vk::TAB, '\t', 0),
        8 | 127 => (vk::BACK, '\u{8}', 0),
        CAPS_LOCK => (vk::CAPITAL, '\0', 0),
        SCROLL_LOCK => (vk::SCROLL, '\0', 0),
        NUM_LOCK => (vk::NUMLOCK, '\0', 0),
        PRINT_SCREEN => (vk::SNAPSHOT, '\0', ENHANCED),
        PAUSE => (vk::PAUSE, '\0', 0),
        MENU => (vk::APPS, '\0', ENHANCED),
        F13..=57387 => (vk::F1 + 12 + (code - F13) as u16, '\0', 0),
        KP_0..=57408 => {
            let digit = (code - KP_0) as u8;
            (vk::NUMPAD0 + u16::from(digit), char::from(b'0' + digit), 0)
        }
        KP_DECIMAL => (vk::DECIMAL, '.', 0),
        KP_DIVIDE => (vk::DIVIDE, '/', ENHANCED),
        KP_MULTIPLY => (vk::MULTIPLY, '*', 0),
        KP_SUBTRACT => (vk::SUBTRACT, '-', 0),
        KP_ADD => (vk::ADD, '+', 0),
        KP_ENTER => (vk::RETURN, '\r', ENHANCED),
        KP_EQUAL => (vk::OEM_PLUS, '=', 0),
        KP_SEPARATOR => (vk::SEPARATOR, ',', 0),
        KP_LEFT => (vk::LEFT, '\0', 0),
        KP_RIGHT => (vk::RIGHT, '\0', 0),
        KP_UP => (vk::UP, '\0', 0),
        KP_DOWN => (vk::DOWN, '\0', 0),
        KP_PAGE_UP => (vk::PRIOR, '\0', 0),
        KP_PAGE_DOWN => (vk::NEXT, '\0', 0),
        KP_HOME => (vk::HOME, '\0', 0),
        KP_END => (vk::END, '\0', 0),
        KP_INSERT => (vk::INSERT, '\0', 0),
        KP_DELETE => (vk::DELETE, '\0', 0),
        KP_BEGIN => (vk::CLEAR, '\0', 0),
        LEFT_SHIFT | RIGHT_SHIFT => (vk::SHIFT, '\0', 0),
        LEFT_CONTROL | RIGHT_CONTROL => (vk::CONTROL, '\0', 0),
        LEFT_ALT | RIGHT_ALT_KEY | ISO_LEVEL3_SHIFT => (vk::MENU, '\0', 0),
        LEFT_SUPER => (vk::LWIN, '\0', 0),
        RIGHT_SUPER => (vk::RWIN, '\0', 0),
        _ => return None,
    };
    Some(key)
}

/// For a modifier key: the bits of the control key state it controls, and the bits it
/// sets while it is held.
fn modifier_key(code: u32) -> Option<(u32, u32, u16)> {
    let both_ctrl = CTRL | RIGHT_CTRL;
    let both_alt = ALT | RIGHT_ALT;
    let entry = match code {
        LEFT_SHIFT => (SHIFT, SHIFT, LEFT_SHIFT_SCAN),
        RIGHT_SHIFT => (SHIFT, SHIFT, RIGHT_SHIFT_SCAN),
        LEFT_CONTROL => (both_ctrl, CTRL, 0),
        RIGHT_CONTROL => (both_ctrl, RIGHT_CTRL | ENHANCED, 0),
        LEFT_ALT => (both_alt, ALT, 0),
        RIGHT_ALT_KEY | ISO_LEVEL3_SHIFT => (both_alt, RIGHT_ALT | ENHANCED, 0),
        _ => return None,
    };
    Some(entry)
}

fn kitty_event(vk_code: u16, ch: char, cks: u32, down: bool) -> InputEvent {
    let mut ev = InputEvent::key(vk_code, 0, ch, down);
    ev.control_key_state = ControlKeyState(cks);
    ev.input_source = SRC_KITTY.to_string();
    ev
}

/// Upper case of `c` if it is a single character.
fn upper(c: char) -> char {
    let mut it = c.to_uppercase();
    match (it.next(), it.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

/// Lower case of `c` if it is a single character.
fn lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

/// Decodes `CSI ... u` (a kitty key event); other uses of the final byte are ignored.
pub(crate) fn decode_u(csi: &Csi, out: &mut Vec<InputEvent>) {
    let Some(code) = csi.sub(0, 0) else {
        return;
    };
    let shifted = csi.sub(0, 1).and_then(char::from_u32);
    let base = csi.sub(0, 2).and_then(char::from_u32);
    let kind = csi.sub(1, 1).unwrap_or(1);
    let down = kind != 3;
    let cks = xterm_mods(csi.param(1).unwrap_or(1));
    let text: Vec<char> = match csi.params.get(2) {
        Some(field) => field
            .iter()
            .flatten()
            .filter_map(|&cp| char::from_u32(cp))
            .collect(),
        None => Vec::new(),
    };

    let mut ev = if code == 0 {
        // No key code, only text (the terminal composed it).
        kitty_event(0, '\0', cks, down)
    } else if let Some((vk_code, ch, extra)) = functional(code) {
        let mut ev = kitty_event(vk_code, ch, cks | extra, down);
        if let Some((group, held, scan)) = modifier_key(code) {
            let others = cks & !group;
            ev.control_key_state = ControlKeyState(if down { others | held } else { others });
            ev.virtual_scan_code = scan;
        }
        ev
    } else if FUNCTIONAL_RANGE.contains(&code) {
        return;
    } else {
        match text_key(code, shifted, base, cks, down) {
            Some(ev) => ev,
            None => return,
        }
    };

    let mut chars = text.into_iter();
    if let Some(first) = chars.next() {
        ev.char_code = first;
    }
    out.push(ev);
    for extra in chars {
        let mut more = char_event(extra);
        more.is_legacy = false;
        more.input_source = SRC_KITTY.to_string();
        more.key_down = down;
        out.push(more);
    }
}

/// Event of a key that produces a character: `code` is its unshifted character.
fn text_key(
    code: u32,
    shifted: Option<char>,
    base: Option<char>,
    cks: u32,
    down: bool,
) -> Option<InputEvent> {
    let c = char::from_u32(code)?;
    if c.is_control() {
        return None;
    }
    let key_char = if c.is_ascii_uppercase() {
        c.to_ascii_lowercase()
    } else {
        c
    };
    let ctrl_held = cks & (CTRL | RIGHT_CTRL) != 0;
    // With Ctrl held the control character comes from the key of the base layout.
    let effective = match base {
        Some(b) if ctrl_held && b.is_ascii() => b,
        _ => key_char,
    };
    let mut ev = key_with_mods(effective, cks);
    ev.is_legacy = false;
    ev.input_source = SRC_KITTY.to_string();
    ev.key_down = down;
    if let Some((physical, _)) = base.and_then(vk_for_char) {
        ev.virtual_key_code = physical;
    }
    if !ctrl_held {
        if cks & SHIFT != 0 {
            if let Some(s) = shifted {
                ev.char_code = s;
            } else if !key_char.is_ascii() {
                ev.char_code = upper(key_char);
            }
        } else if cks & CAPS != 0 && key_char.is_alphabetic() {
            ev.char_code = upper(key_char);
        }
    }
    ev.unshifted_char = key_char;
    Some(ev)
}

// ----------------------------------------------------------------------------------
// Encoding

/// How a functional key is written.
enum Target {
    /// `CSI 1 ; mods letter`
    Letter(u8),
    /// `CSI number ; mods ~`
    Tilde(u32),
    /// `CSI code ; mods u`
    Code(u32),
}

/// Unshifted character of a virtual key on a US layout.
fn char_for_vk(code: u16) -> Option<char> {
    let c = match code {
        0x41..=0x5A => char::from(code as u8).to_ascii_lowercase(),
        0x30..=0x39 => char::from(code as u8),
        vk::SPACE => ' ',
        vk::OEM_1 => ';',
        vk::OEM_PLUS => '=',
        vk::OEM_COMMA => ',',
        vk::OEM_MINUS => '-',
        vk::OEM_PERIOD => '.',
        vk::OEM_2 => '/',
        vk::OEM_3 => '`',
        vk::OEM_4 => '[',
        vk::OEM_5 => '\\',
        vk::OEM_6 => ']',
        vk::OEM_7 => '\'',
        _ => return None,
    };
    Some(c)
}

/// Encoding of a key that has a functional key number or a legacy sequence.
fn functional_target(code: u16, enhanced: bool) -> Option<Target> {
    const FN_TILDES: [u32; 8] = [15, 17, 18, 19, 20, 21, 23, 24];
    let target = match code {
        vk::UP if enhanced => Target::Letter(b'A'),
        vk::DOWN if enhanced => Target::Letter(b'B'),
        vk::RIGHT if enhanced => Target::Letter(b'C'),
        vk::LEFT if enhanced => Target::Letter(b'D'),
        vk::HOME if enhanced => Target::Letter(b'H'),
        vk::END if enhanced => Target::Letter(b'F'),
        vk::PRIOR if enhanced => Target::Tilde(5),
        vk::NEXT if enhanced => Target::Tilde(6),
        vk::INSERT if enhanced => Target::Tilde(2),
        vk::DELETE if enhanced => Target::Tilde(3),
        vk::UP => Target::Code(KP_UP),
        vk::DOWN => Target::Code(KP_DOWN),
        vk::RIGHT => Target::Code(KP_RIGHT),
        vk::LEFT => Target::Code(KP_LEFT),
        vk::HOME => Target::Code(KP_HOME),
        vk::END => Target::Code(KP_END),
        vk::PRIOR => Target::Code(KP_PAGE_UP),
        vk::NEXT => Target::Code(KP_PAGE_DOWN),
        vk::INSERT => Target::Code(KP_INSERT),
        vk::DELETE => Target::Code(KP_DELETE),
        vk::CLEAR => Target::Code(KP_BEGIN),
        vk::F1..=vk::F24 => {
            let n = usize::from(code - vk::F1);
            match n {
                0 => Target::Letter(b'P'),
                1 => Target::Letter(b'Q'),
                2 => Target::Tilde(13),
                3 => Target::Letter(b'S'),
                4..=11 => Target::Tilde(FN_TILDES[n - 4]),
                _ => Target::Code(F13 + (n - 12) as u32),
            }
        }
        vk::NUMPAD0..=vk::NUMPAD9 => Target::Code(KP_0 + u32::from(code - vk::NUMPAD0)),
        vk::DECIMAL => Target::Code(KP_DECIMAL),
        vk::DIVIDE => Target::Code(KP_DIVIDE),
        vk::MULTIPLY => Target::Code(KP_MULTIPLY),
        vk::SUBTRACT => Target::Code(KP_SUBTRACT),
        vk::ADD => Target::Code(KP_ADD),
        vk::SEPARATOR => Target::Code(KP_SEPARATOR),
        vk::RETURN if enhanced => Target::Code(KP_ENTER),
        vk::CAPITAL => Target::Code(CAPS_LOCK),
        vk::SCROLL => Target::Code(SCROLL_LOCK),
        vk::NUMLOCK => Target::Code(NUM_LOCK),
        vk::SNAPSHOT => Target::Code(PRINT_SCREEN),
        vk::PAUSE => Target::Code(PAUSE),
        vk::APPS => Target::Code(MENU),
        vk::ESCAPE => Target::Code(27),
        _ => return None,
    };
    Some(target)
}

/// Functional key number of a modifier key event.
fn modifier_code(ev: &InputEvent) -> Option<u32> {
    let cks = ev.control_key_state.0;
    let code = match ev.virtual_key_code {
        vk::SHIFT | vk::LSHIFT if ev.virtual_scan_code != RIGHT_SHIFT_SCAN => LEFT_SHIFT,
        vk::SHIFT | vk::LSHIFT | vk::RSHIFT => RIGHT_SHIFT,
        vk::CONTROL | vk::LCONTROL if cks & RIGHT_CTRL == 0 => LEFT_CONTROL,
        vk::CONTROL | vk::LCONTROL | vk::RCONTROL => RIGHT_CONTROL,
        vk::MENU | vk::LMENU if cks & RIGHT_ALT == 0 => LEFT_ALT,
        vk::MENU | vk::LMENU | vk::RMENU => RIGHT_ALT_KEY,
        vk::LWIN => LEFT_SUPER,
        vk::RWIN => RIGHT_SUPER,
        _ => return None,
    };
    Some(code)
}

/// Whether `c` is a character that a key can type (not a control character).
fn printable(c: char) -> bool {
    !c.is_control()
}

/// Win32 control key state to the kitty modifier bit mask (without the +1).
fn mod_bits(cks: u32) -> u32 {
    let mut bits = 0;
    if cks & SHIFT != 0 {
        bits |= 1;
    }
    if cks & (ALT | RIGHT_ALT) != 0 {
        bits |= 2;
    }
    if cks & (CTRL | RIGHT_CTRL) != 0 {
        bits |= 4;
    }
    if cks & CAPS != 0 {
        bits |= 64;
    }
    if cks & NUM != 0 {
        bits |= 128;
    }
    bits
}

/// `modifiers[:event]` of a sequence.
fn mods_field(mods: u32, kind: u32) -> String {
    if kind == 1 {
        mods.to_string()
    } else {
        format!("{mods}:{kind}")
    }
}

/// Writes the sequence for a functional key.
fn write_target(target: &Target, mods: u32, kind: u32) -> Vec<u8> {
    let plain = mods == 1 && kind == 1;
    let text = match *target {
        Target::Letter(letter) if plain => format!("\x1b[{}", char::from(letter)),
        Target::Letter(letter) => {
            format!("\x1b[1;{}{}", mods_field(mods, kind), char::from(letter))
        }
        Target::Tilde(n) if plain => format!("\x1b[{n}~"),
        Target::Tilde(n) => format!("\x1b[{n};{}~", mods_field(mods, kind)),
        Target::Code(n) if plain => format!("\x1b[{n}u"),
        Target::Code(n) => format!("\x1b[{n};{}u", mods_field(mods, kind)),
    };
    text.into_bytes()
}

/// Converts a key event to the bytes a terminal speaking the kitty keyboard protocol sends
/// when the progressive enhancement `flags` (the constants of this module) are active.
///
/// Returns `None` if the event is not a key event, if [`DISAMBIGUATE`] is off (the legacy
/// encoding is not produced here), if the protocol would not report the event with these
/// flags (releases without [`EVENT_TYPES`], text keys without [`ALL_KEYS`] on release,
/// modifier keys without [`ALL_KEYS`]), or if the key has no kitty encoding.
pub fn encode(ev: &InputEvent, flags: u8) -> Option<Vec<u8>> {
    if ev.event_type != EventType::Key || flags & DISAMBIGUATE == 0 {
        return None;
    }
    let all = flags & ALL_KEYS != 0;
    let press = ev.key_down;
    if !press && flags & EVENT_TYPES == 0 {
        return None;
    }
    let cks = ev.control_key_state.0;
    let mods = 1 + mod_bits(cks);
    let kind = if press { 1 } else { 3 };
    let enhanced = cks & ENHANCED != 0;
    let held = cks & (SHIFT | ALT | RIGHT_ALT | CTRL | RIGHT_CTRL) != 0;

    if let Some(code) = modifier_code(ev) {
        if !all {
            return None;
        }
        return Some(write_target(&Target::Code(code), mods, kind));
    }
    // Enter, Tab and Backspace stay legacy bytes until modifiers or ALL_KEYS are involved.
    let legacy_byte = match ev.virtual_key_code {
        vk::RETURN if !enhanced => Some(13),
        vk::TAB => Some(9),
        vk::BACK => Some(127),
        _ => None,
    };
    if let Some(byte) = legacy_byte {
        if !all && !press {
            return None;
        }
        if !all && !held {
            return Some(vec![byte]);
        }
        return Some(write_target(&Target::Code(u32::from(byte)), mods, kind));
    }
    if let Some(target) = functional_target(ev.virtual_key_code, enhanced) {
        return Some(write_target(&target, mods, kind));
    }
    encode_text_key(ev, flags, mods, kind)
}

/// Encodes a key that produces a character.
fn encode_text_key(ev: &InputEvent, flags: u8, mods: u32, kind: u32) -> Option<Vec<u8>> {
    let all = flags & ALL_KEYS != 0;
    let press = ev.key_down;
    let cks = ev.control_key_state.0;
    let ch = ev.char_code;
    let from_vk = char_for_vk(ev.virtual_key_code);
    let code_char = if printable(ev.unshifted_char) {
        ev.unshifted_char
    } else if printable(ch) && !ch.is_ascii() {
        lower(ch)
    } else if let Some(c) = from_vk {
        c
    } else if printable(ch) {
        lower(ch)
    } else {
        return None;
    };
    let no_ctrl_alt = cks & (ALT | RIGHT_ALT | CTRL | RIGHT_CTRL) == 0;
    let has_text = printable(ch) && no_ctrl_alt;

    if !all {
        if !press {
            return None;
        }
        if has_text {
            return Some(ch.to_string().into_bytes());
        }
    }

    let mut key = u32::from(code_char).to_string();
    if flags & ALTERNATE_KEYS != 0 {
        let shifted = (cks & SHIFT != 0 && printable(ch) && ch != code_char).then_some(ch);
        let base = from_vk.filter(|&b| b != code_char);
        if shifted.is_some() || base.is_some() {
            key.push(':');
            if let Some(s) = shifted {
                key.push_str(&u32::from(s).to_string());
            }
            if let Some(b) = base {
                key.push(':');
                key.push_str(&u32::from(b).to_string());
            }
        }
    }
    let mut seq = format!("\x1b[{key}");
    let with_text = all && flags & ASSOCIATED_TEXT != 0 && press && has_text;
    if with_text || !(mods == 1 && kind == 1) {
        seq.push(';');
        if !(mods == 1 && kind == 1) {
            seq.push_str(&mods_field(mods, kind));
        }
    }
    if with_text {
        seq.push(';');
        seq.push_str(&u32::from(ch).to_string());
    }
    seq.push('u');
    Some(seq.into_bytes())
}
