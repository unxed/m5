//! Mouse reports in the SGR encoding (xterm mode 1006): `CSI < b ; x ; y M` for a press or
//! motion and `CSI < b ; x ; y m` for a release, with 1-based cell coordinates.
//!
//! The button code `b` is: bits 0-1 the button (0 left, 1 middle, 2 right, 3 none), +4 Shift,
//! +8 Alt, +16 Ctrl, +32 motion, +64 wheel (then 0 up, 1 down, 2 left, 3 right), +128 for the
//! additional buttons 8 and 9 (xterm ctlseqs, "Mouse Tracking" and "SGR Mouse Mode").
//!
//! The reports become Win32 mouse records: the button state is the set of buttons held after
//! the event (so a release is an event in which the bit is no longer set), a wheel step puts
//! +1 or -1 in the high half of the button state as far2l does and sets `wheel_direction`.
//! Terminals do not report double clicks, so `DOUBLE_CLICK` is never set here.

use super::csi::Csi;
use super::keys::{ALT, CTRL, SHIFT};
use crate::key::{ControlKeyState, InputEvent, MouseButtonState, MouseEventFlags};

/// Enables button and drag reports in the SGR encoding.
pub const ENABLE: &[u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1006h";
/// Like [`ENABLE`], and also reports motion with no button held.
pub const ENABLE_MOTION: &[u8] = b"\x1b[?1000h\x1b[?1003h\x1b[?1006h";
/// Turns all of it off.
pub const DISABLE: &[u8] = b"\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l";

const SGR_SHIFT: u32 = 4;
const SGR_ALT: u32 = 8;
const SGR_CTRL: u32 = 16;
const SGR_MOTION: u32 = 32;
const SGR_WHEEL: u32 = 64;
const SGR_EXTRA: u32 = 128;

/// Wheel delta in the high half of the button state.
const WHEEL_UP: u32 = 0x0001_0000;
const WHEEL_DOWN: u32 = 0xFFFF_0000;

/// Win32 button bit of the button number in the low bits of `b` (and the extra bit).
fn button_bit(code: u32) -> u32 {
    match (code & SGR_EXTRA != 0, code & 3) {
        (false, 0) => MouseButtonState::FROM_LEFT_1ST_BUTTON_PRESSED,
        (false, 1) => MouseButtonState::FROM_LEFT_2ND_BUTTON_PRESSED,
        (false, 2) => MouseButtonState::RIGHTMOST_BUTTON_PRESSED,
        (true, 0) => MouseButtonState::FROM_LEFT_3RD_BUTTON_PRESSED,
        (true, 1) => MouseButtonState::FROM_LEFT_4TH_BUTTON_PRESSED,
        _ => 0,
    }
}

/// A 1-based coordinate as a 0-based one.
fn cell(value: u32) -> i16 {
    i16::try_from(value.saturating_sub(1)).unwrap_or(i16::MAX)
}

/// Decodes `CSI < b ; x ; y M|m`. `held` is the set of buttons held, kept between calls.
pub(crate) fn decode_sgr(csi: &Csi, out: &mut Vec<InputEvent>, held: &mut u32) {
    if csi.final_byte != b'M' && csi.final_byte != b'm' {
        return;
    }
    let (Some(code), Some(x), Some(y)) = (csi.param(0), csi.param(1), csi.param(2)) else {
        return;
    };
    let release = csi.final_byte == b'm';
    let mut cks = 0;
    if code & SGR_SHIFT != 0 {
        cks |= SHIFT;
    }
    if code & SGR_ALT != 0 {
        cks |= ALT;
    }
    if code & SGR_CTRL != 0 {
        cks |= CTRL;
    }
    let mut flags = 0;
    let mut wheel = 0;
    let mut delta = 0;
    if code & SGR_WHEEL != 0 {
        if release {
            return;
        }
        let (vertical, direction) = match code & 3 {
            0 => (true, 1),
            1 => (true, -1),
            2 => (false, -1),
            _ => (false, 1),
        };
        flags = if vertical {
            MouseEventFlags::WHEELED
        } else {
            MouseEventFlags::HWHEELED
        };
        wheel = direction;
        delta = if direction > 0 { WHEEL_UP } else { WHEEL_DOWN };
    } else {
        let bit = button_bit(code);
        if release {
            *held &= !bit;
        } else {
            *held |= bit;
        }
        if code & SGR_MOTION != 0 {
            flags = MouseEventFlags::MOVED;
        }
    }
    let mut ev = InputEvent::mouse(cell(x), cell(y), *held | delta, flags);
    ev.control_key_state = ControlKeyState(cks);
    ev.wheel_direction = wheel;
    ev.input_source = "sgr_mouse".to_string();
    out.push(ev);
}
