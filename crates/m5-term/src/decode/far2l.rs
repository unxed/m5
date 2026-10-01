//! The far2l terminal extensions: keyboard, mouse and size events (and the replies and
//! acknowledgement) that a far2l-aware terminal sends as APC strings.
//!
//! An event is `ESC _ f2l <base64> BEL` (or ending with `ESC \`). The base64 text is a
//! binary *stack*: the last byte is the event code and the arguments are popped from the end,
//! little endian. This follows `VTExts.md` of far2l (sections 4 and 6): `K`/`k` key down/up
//! (character u32, control key state u32, scan code u16, virtual key u16, repeat count u16),
//! `C`/`c` the compact forms (character u16, control key state u16, virtual key u8),
//! `M` mouse (flags u32, control key state u32, button state u32, y i16, x i16), `m` the
//! compact mouse form, `S` terminal size (height u16, width u16). Unknown codes and events
//! whose stack is too short are dropped.
//!
//! Other far2l strings become events of type [`EventType::Far2l`], with the command in
//! `far2l_command` and the decoded stack in `far2l_data`: `ESC _ far2lok` is `"ok"`,
//! `far2l1` and `far2l0` are `"enable"` and `"disable"`, `ESC _ far2l:<base64>` is
//! `"interact"` (a request) and `ESC _ far2l<base64>` is `"reply"`.
//!
//! Deviations from the strict text of the specification: a colon after `f2l` is tolerated
//! (some implementations send it, far2l itself would read an empty stack) and the compact key
//! forms leave the scan code at 0 (far2l derives it from the virtual key with a Windows API).

use super::keys::SRC_FAR2L;
use crate::key::{ControlKeyState, EventType, InputEvent, MouseEventFlags};

/// Asks the terminal to switch the far2l extensions on; a far2l terminal answers `ESC _ far2lok ESC \`.
pub const QUERY: &[u8] = b"\x1b_far2l1\x1b\\";
/// Switches the far2l extensions off.
pub const DISABLE: &[u8] = b"\x1b_far2l0\x1b\\";

/// Value of a base64 digit.
fn base64_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decodes standard base64 the way far2l does: padding is optional and decoding stops at the
/// first character that is not part of the alphabet (so at `=`).
pub fn base64_decode(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3 + 2);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for &b in text {
        let Some(v) = base64_value(b) else {
            break;
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// A byte stack read from the end.
struct Stack(Vec<u8>);

impl Stack {
    fn pop_u8(&mut self) -> Option<u8> {
        self.0.pop()
    }

    fn pop_u16(&mut self) -> Option<u16> {
        let hi = self.0.pop()?;
        let lo = self.0.pop()?;
        Some((u16::from(hi) << 8) | u16::from(lo))
    }

    fn pop_u32(&mut self) -> Option<u32> {
        let hi = self.pop_u16()?;
        let lo = self.pop_u16()?;
        Some((u32::from(hi) << 16) | u32::from(lo))
    }
}

/// Decodes the body of an APC string (the bytes between `ESC _` and the terminator).
pub(crate) fn decode(body: &[u8], out: &mut Vec<InputEvent>) {
    if let Some(rest) = body.strip_prefix(b"f2l") {
        let rest = rest.strip_prefix(b":").unwrap_or(rest);
        if let Some(ev) = decode_event(Stack(base64_decode(rest))) {
            out.push(ev);
        }
    } else if let Some(rest) = body.strip_prefix(b"far2l") {
        if let Some(ev) = decode_control(rest) {
            out.push(ev);
        }
    }
}

fn control_event(command: &str, data: Vec<u8>) -> InputEvent {
    let mut ev = InputEvent::resize();
    ev.event_type = EventType::Far2l;
    ev.far2l_command = command.to_string();
    ev.far2l_data = data;
    ev.input_source = SRC_FAR2L.to_string();
    ev
}

/// The strings that start with `far2l`: acknowledgement, switches, requests and replies.
fn decode_control(rest: &[u8]) -> Option<InputEvent> {
    match rest {
        b"ok" => Some(control_event("ok", Vec::new())),
        b"1" => Some(control_event("enable", Vec::new())),
        b"0" => Some(control_event("disable", Vec::new())),
        [] => None,
        [b'#', ..] | [b'_', ..] => None,
        [b':', request @ ..] => Some(control_event("interact", base64_decode(request))),
        reply => Some(control_event("reply", base64_decode(reply))),
    }
}

fn decode_event(mut stack: Stack) -> Option<InputEvent> {
    let code = stack.pop_u8()?;
    match code {
        b'K' | b'k' => {
            let ch = stack.pop_u32()?;
            let cks = stack.pop_u32()?;
            let scan = stack.pop_u16()?;
            let key = stack.pop_u16()?;
            let repeat = stack.pop_u16()?;
            Some(key_event(code == b'K', ch, cks, scan, key, repeat))
        }
        b'C' | b'c' => {
            let ch = stack.pop_u16()?;
            let cks = stack.pop_u16()?;
            let key = stack.pop_u8()?;
            Some(key_event(
                code == b'C',
                u32::from(ch),
                u32::from(cks),
                0,
                u16::from(key),
                1,
            ))
        }
        b'M' => {
            let flags = stack.pop_u32()?;
            let cks = stack.pop_u32()?;
            let buttons = stack.pop_u32()?;
            let y = stack.pop_u16()? as i16;
            let x = stack.pop_u16()? as i16;
            Some(mouse_event(flags, cks, buttons, x, y))
        }
        b'm' => {
            let flags = u32::from(stack.pop_u8()?);
            let cks = u32::from(stack.pop_u8()?);
            let packed = u32::from(stack.pop_u16()?);
            let buttons = (packed & 0xFF) | ((packed & 0xFF00) << 8);
            let y = stack.pop_u16()? as i16;
            let x = stack.pop_u16()? as i16;
            Some(mouse_event(flags, cks, buttons, x, y))
        }
        b'S' => {
            let height = stack.pop_u16()?;
            let width = stack.pop_u16()?;
            let mut ev = InputEvent::resize_to(width, height);
            ev.input_source = SRC_FAR2L.to_string();
            Some(ev)
        }
        _ => None,
    }
}

fn key_event(down: bool, ch: u32, cks: u32, scan: u16, key: u16, repeat: u16) -> InputEvent {
    let c = char::from_u32(ch).unwrap_or('\0');
    let mut ev = InputEvent::key(key, scan, c, down);
    ev.repeat_count = repeat;
    ev.control_key_state = ControlKeyState(cks);
    ev.input_source = SRC_FAR2L.to_string();
    ev
}

fn mouse_event(flags: u32, cks: u32, buttons: u32, x: i16, y: i16) -> InputEvent {
    let mut ev = InputEvent::mouse(x, y, buttons, flags);
    ev.control_key_state = ControlKeyState(cks);
    ev.input_source = SRC_FAR2L.to_string();
    if flags & (MouseEventFlags::WHEELED | MouseEventFlags::HWHEELED) != 0 {
        // The high half of the button state is the signed wheel delta; only its sign matters.
        let delta = (buttons >> 16) as u16 as i16;
        ev.wheel_direction = i32::from(delta.signum());
    }
    ev
}
