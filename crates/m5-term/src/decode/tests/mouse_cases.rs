//! Tests of SGR mouse reports (xterm ctlseqs, "SGR Mouse Mode (1006)"): `CSI < b ; x ; y M`
//! press or motion, `m` release, 1-based coordinates.

use super::*;
use crate::decode::mouse::{DISABLE, ENABLE, ENABLE_MOTION};

const LEFT: u32 = 0x0001;
const RIGHT: u32 = 0x0002;
const MIDDLE: u32 = 0x0004;
const MOVED: u32 = 0x0001;
const WHEELED: u32 = 0x0004;
const HWHEELED: u32 = 0x0008;

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

/// Position, button state, event flags and control key state of the single mouse event.
fn mouse(bytes: &[u8]) -> (i16, i16, u32, u32, u32) {
    let evs = events(bytes);
    assert_eq!(evs.len(), 1, "{bytes:?} gave {evs:?}");
    let ev = &evs[0];
    assert_eq!(ev.event_type, EventType::Mouse);
    (
        ev.mouse_x,
        ev.mouse_y,
        ev.button_state,
        ev.mouse_event_flags,
        ev.control_key_state.0,
    )
}

#[test]
fn press_and_release_of_each_button() {
    assert_eq!(mouse(b"\x1b[<0;10;5M"), (9, 4, LEFT, 0, 0));
    assert_eq!(mouse(b"\x1b[<0;10;5m"), (9, 4, 0, 0, 0));
    assert_eq!(mouse(b"\x1b[<1;1;1M"), (0, 0, MIDDLE, 0, 0));
    assert_eq!(mouse(b"\x1b[<1;1;1m"), (0, 0, 0, 0, 0));
    assert_eq!(mouse(b"\x1b[<2;80;24M"), (79, 23, RIGHT, 0, 0));
    assert_eq!(mouse(b"\x1b[<2;80;24m"), (79, 23, 0, 0, 0));
}

#[test]
fn event_fields() {
    let ev = &events(b"\x1b[<0;10;5M")[0];
    assert_eq!(ev.input_source, "sgr_mouse");
    assert_eq!(ev.wheel_direction, 0);
}

#[test]
fn held_buttons_accumulate_across_reports() {
    let evs = events(b"\x1b[<0;1;1M\x1b[<2;1;1M\x1b[<0;1;1m\x1b[<2;1;1m");
    let states: Vec<u32> = evs.iter().map(|e| e.button_state).collect();
    assert_eq!(states, [LEFT, LEFT | RIGHT, RIGHT, 0]);
}

#[test]
fn state_is_kept_between_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[<0;1;1M", &mut out);
    d.feed(b"\x1b[<2;2;2", &mut out);
    assert_eq!(out.len(), 1);
    d.feed(b"M", &mut out);
    assert_eq!(out[1].button_state, LEFT | RIGHT);
}

#[test]
fn drag_and_hover() {
    // 32 is motion with the left button held, 35 is motion with no button.
    let evs = events(b"\x1b[<0;10;5M\x1b[<32;11;5M\x1b[<0;11;5m\x1b[<35;3;4M");
    assert_eq!(evs.len(), 4);
    assert_eq!(
        (
            evs[1].mouse_x,
            evs[1].button_state,
            evs[1].mouse_event_flags
        ),
        (10, LEFT, MOVED)
    );
    assert_eq!((evs[2].button_state, evs[2].mouse_event_flags), (0, 0));
    assert_eq!(
        (evs[3].mouse_x, evs[3].mouse_y, evs[3].button_state),
        (2, 3, 0)
    );
    assert_eq!(evs[3].mouse_event_flags, MOVED);
}

#[test]
fn modifiers() {
    assert_eq!(mouse(b"\x1b[<4;1;1M"), (0, 0, LEFT, 0, SHIFT));
    assert_eq!(mouse(b"\x1b[<8;1;1M"), (0, 0, LEFT, 0, ALT));
    assert_eq!(mouse(b"\x1b[<16;1;1M"), (0, 0, LEFT, 0, CTRL));
    assert_eq!(mouse(b"\x1b[<20;1;1M"), (0, 0, LEFT, 0, SHIFT | CTRL));
    assert_eq!(mouse(b"\x1b[<28;1;1M"), (0, 0, LEFT, 0, SHIFT | ALT | CTRL));
}

#[test]
fn alt_prefix_adds_alt() {
    assert_eq!(mouse(b"\x1b\x1b[<0;1;1M"), (0, 0, LEFT, 0, ALT));
}

#[test]
fn wheel() {
    let up = &events(b"\x1b[<64;5;6M")[0];
    assert_eq!((up.mouse_x, up.mouse_y), (4, 5));
    assert_eq!(up.mouse_event_flags, WHEELED);
    assert_eq!(up.button_state, 0x0001_0000);
    assert_eq!(up.wheel_direction, 1);
    let down = &events(b"\x1b[<65;5;6M")[0];
    assert_eq!(down.mouse_event_flags, WHEELED);
    assert_eq!(down.button_state, 0xFFFF_0000);
    assert_eq!(down.wheel_direction, -1);
    let left = &events(b"\x1b[<66;1;1M")[0];
    assert_eq!(left.mouse_event_flags, HWHEELED);
    assert_eq!(left.button_state, 0xFFFF_0000);
    assert_eq!(left.wheel_direction, -1);
    let right = &events(b"\x1b[<67;1;1M")[0];
    assert_eq!(right.mouse_event_flags, HWHEELED);
    assert_eq!(right.wheel_direction, 1);
    // Ctrl+wheel, and wheel with the left button held.
    assert_eq!(mouse(b"\x1b[<80;1;1M"), (0, 0, 0x0001_0000, WHEELED, CTRL));
    let evs = events(b"\x1b[<0;1;1M\x1b[<65;1;1M");
    assert_eq!(evs[1].button_state, 0xFFFF_0000 | LEFT);
    // Wheel reports have no release.
    assert!(events(b"\x1b[<64;1;1m").is_empty());
}

#[test]
fn additional_buttons() {
    assert_eq!(mouse(b"\x1b[<128;1;1M"), (0, 0, 0x0008, 0, 0));
    assert_eq!(mouse(b"\x1b[<129;1;1M"), (0, 0, 0x0010, 0, 0));
}

#[test]
fn coordinates() {
    assert_eq!(mouse(b"\x1b[<0;300;200M"), (299, 199, LEFT, 0, 0));
    // A zero coordinate (not valid in the protocol) is clamped to the first cell.
    assert_eq!(mouse(b"\x1b[<0;0;0M"), (0, 0, LEFT, 0, 0));
    // Coordinates beyond the range of i16 saturate.
    assert_eq!(mouse(b"\x1b[<0;99999;40000M").0, i16::MAX);
}

#[test]
fn malformed_reports_are_dropped() {
    assert!(events(b"\x1b[<0;1M").is_empty());
    assert!(events(b"\x1b[<0;1;1X").is_empty());
    assert!(events(b"\x1b[<M").is_empty());
    assert!(events(b"\x1b[<0;1;1 M").is_empty());
}

#[test]
fn mouse_among_keys() {
    let evs = events(b"a\x1b[<0;2;3Mb");
    assert_eq!(evs.len(), 3);
    assert_eq!(evs[0].char_code, 'a');
    assert_eq!(evs[1].event_type, EventType::Mouse);
    assert_eq!(evs[2].char_code, 'b');
}

#[test]
fn mode_switch_sequences() {
    assert_eq!(ENABLE, b"\x1b[?1000h\x1b[?1002h\x1b[?1006h");
    assert_eq!(ENABLE_MOTION, b"\x1b[?1000h\x1b[?1003h\x1b[?1006h");
    assert_eq!(DISABLE, b"\x1b[?1006l\x1b[?1003l\x1b[?1002l\x1b[?1000l");
}
