//! Tests of the far2l terminal extensions. Vectors named "VTExts 7.8" are the byte examples
//! of section 7.8 of far2l's `VTExts.md` (produced by the code of far2l); the others were
//! built by hand from the stack layout of sections 4.3 and 6.

use super::*;
use crate::decode::far2l::base64_decode;

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

fn first(bytes: &[u8]) -> InputEvent {
    let mut all = events(bytes);
    assert_eq!(all.len(), 1, "{bytes:?} gave {all:?}");
    all.remove(0)
}

#[test]
fn base64_rfc4648_vectors() {
    assert_eq!(base64_decode(b""), b"");
    assert_eq!(base64_decode(b"Zg=="), b"f");
    assert_eq!(base64_decode(b"Zm8="), b"fo");
    assert_eq!(base64_decode(b"Zm9v"), b"foo");
    assert_eq!(base64_decode(b"Zm9vYg=="), b"foob");
    assert_eq!(base64_decode(b"Zm9vYmE="), b"fooba");
    assert_eq!(base64_decode(b"Zm9vYmFy"), b"foobar");
}

#[test]
fn base64_as_far2l_reads_it() {
    // Padding is optional.
    assert_eq!(base64_decode(b"Zg"), b"f");
    assert_eq!(base64_decode(b"Zm8"), b"fo");
    // Decoding stops at the first character outside the alphabet.
    assert_eq!(base64_decode(b"Zm9v=Zm9v"), b"foo");
    assert_eq!(base64_decode(b"Zm9v\nZm9v"), b"foo");
    assert_eq!(base64_decode(b":Zm9v"), b"");
    // A lone trailing digit carries no whole byte.
    assert_eq!(base64_decode(b"Zm9vY"), b"foo");
    // The last two characters of the alphabet.
    assert_eq!(base64_decode(b"+/8="), [0xFB, 0xFF]);
}

#[test]
fn key_down_full_form() {
    // VTExts 7.8: the letter a, VK 0x41, scan 0x1E.
    let ev = first(b"\x1b_f2lAQBBAB4AAAAAAGEAAABL\x07");
    assert_eq!(sig(&ev), (0x41, 'a', 0, true));
    assert_eq!(ev.virtual_scan_code, 0x1E);
    assert_eq!(ev.repeat_count, 1);
    assert_eq!(ev.input_source, "far2l");
    assert!(!ev.is_legacy);
}

#[test]
fn key_up_full_form() {
    let ev = first(b"\x1b_f2lAQBBAB4AAAAAAGEAAABr\x07");
    assert_eq!(sig(&ev), (0x41, 'a', 0, false));
    assert_eq!(ev.virtual_scan_code, 0x1E);
}

#[test]
fn key_with_state_and_repeat_count() {
    // Shift+A, repeat count 2.
    let ev = first(b"\x1b_f2lAgBBAB4AEAAAAEEAAABL\x07");
    assert_eq!(sig(&ev), (0x41, 'A', SHIFT, true));
    assert_eq!(ev.repeat_count, 2);
    // Ctrl+Left (left Ctrl and enhanced key), no character.
    let ev = first(b"\x1b_f2lAQAlAEsACAEAAAAAAABL\x07");
    assert_eq!(sig(&ev), (0x25, '\0', CTRL | ENH, true));
    assert_eq!(ev.virtual_scan_code, 0x4B);
}

#[test]
fn key_with_a_character_outside_the_bmp() {
    let ev = first(b"\x1b_f2lAQAAAAAAAAAAAAD2AQBL\x07");
    assert_eq!(sig(&ev), (0, '😀', 0, true));
}

#[test]
fn compact_key_events() {
    // VTExts 7.8: the same press of a in the compact form.
    let ev = first(b"\x1b_f2lQQAAYQBD\x07");
    assert_eq!(sig(&ev), (0x41, 'a', 0, true));
    assert_eq!(ev.virtual_scan_code, 0);
    assert_eq!(ev.repeat_count, 1);
    assert_eq!(sig(&first(b"\x1b_f2lQQAAYQBj\x07")), (0x41, 'a', 0, false));
    // Ctrl+A: character 1, control key state 8.
    assert_eq!(
        sig(&first(b"\x1b_f2lQQgAAQBD\x07")),
        (0x41, '\u{1}', CTRL, true)
    );
}

#[test]
fn terminators_bel_and_st_and_colon() {
    let by_st = first(b"\x1b_f2lQQAAYQBD\x1b\\");
    assert_eq!(sig(&by_st), (0x41, 'a', 0, true));
    // A colon after f2l is tolerated.
    let colon = first(b"\x1b_f2l:QQAAYQBD\x07");
    assert_eq!(sig(&colon), (0x41, 'a', 0, true));
}

#[test]
fn mouse_compact_press() {
    // VTExts 7.8: left button pressed at column 10, row 5.
    let ev = first(b"\x1b_f2lCgAFAAEAAABt\x07");
    assert_eq!(ev.event_type, EventType::Mouse);
    assert_eq!((ev.mouse_x, ev.mouse_y), (10, 5));
    assert_eq!(ev.button_state, 1);
    assert_eq!(ev.mouse_event_flags, 0);
    assert_eq!(ev.wheel_direction, 0);
    assert_eq!(ev.input_source, "far2l");
}

#[test]
fn mouse_wheel_down_full_form() {
    // VTExts 7.8: wheel turned down at the same place.
    let ev = first(b"\x1b_f2lCgAFAAAA//8AAAAABAAAAE0=\x07");
    assert_eq!(ev.event_type, EventType::Mouse);
    assert_eq!((ev.mouse_x, ev.mouse_y), (10, 5));
    assert_eq!(ev.mouse_event_flags, 4);
    assert_eq!(ev.button_state, 0xFFFF_0000);
    assert_eq!(ev.wheel_direction, -1);
}

#[test]
fn mouse_wheel_up_compact_and_horizontal() {
    // Compact form: the delta 1 in the high half of the state is packed into 0x0100.
    let ev = first(b"\x1b_f2lAwAEAAABAARt\x07");
    assert_eq!((ev.mouse_x, ev.mouse_y), (3, 4));
    assert_eq!(ev.mouse_event_flags, 4);
    assert_eq!(ev.button_state, 0x0001_0000);
    assert_eq!(ev.wheel_direction, 1);
    // Horizontal wheel to the right with Shift held.
    let ev = first(b"\x1b_f2lCQAHAAAAAQAQAAAACAAAAE0=\x07");
    assert_eq!((ev.mouse_x, ev.mouse_y), (9, 7));
    assert_eq!(ev.mouse_event_flags, 8);
    assert_eq!(ev.control_key_state.0, SHIFT);
    assert_eq!(ev.wheel_direction, 1);
}

#[test]
fn mouse_motion_and_negative_coordinates() {
    let ev = first(b"\x1b_f2lAwACAAEAAAAAAAAAAQAAAE0=\x07");
    assert_eq!((ev.mouse_x, ev.mouse_y), (3, 2));
    assert_eq!(ev.mouse_event_flags, 1);
    assert_eq!(ev.button_state, 1);
    let ev = first(b"\x1b_f2l/v///wAAAAFt\x07");
    assert_eq!((ev.mouse_x, ev.mouse_y), (-2, -1));
    assert_eq!(ev.mouse_event_flags, 1);
}

#[test]
fn terminal_size() {
    // VTExts 7.8: 80 columns by 25 rows (height is popped first).
    let ev = first(b"\x1b_f2lUAAZAFM=\x07");
    assert_eq!(ev.event_type, EventType::Resize);
    assert_eq!((ev.term_width, ev.term_height), (80, 25));
}

#[test]
fn acknowledgement_requests_and_replies() {
    let ok = first(b"\x1b_far2lok\x07");
    assert_eq!(ok.event_type, EventType::Far2l);
    assert_eq!(ok.far2l_command, "ok");
    assert!(ok.far2l_data.is_empty());
    assert_eq!(first(b"\x1b_far2lok\x1b\\").far2l_command, "ok");
    assert_eq!(first(b"\x1b_far2l1\x1b\\").far2l_command, "enable");
    assert_eq!(first(b"\x1b_far2l0\x07").far2l_command, "disable");
    let request = first(b"\x1b_far2l:AQID\x07");
    assert_eq!(request.far2l_command, "interact");
    assert_eq!(request.far2l_data, [1, 2, 3]);
    let reply = first(b"\x1b_far2lAQID\x07");
    assert_eq!(reply.far2l_command, "reply");
    assert_eq!(reply.far2l_data, [1, 2, 3]);
    assert_eq!(reply.input_source, "far2l");
}

#[test]
fn strings_that_are_dropped() {
    // Host identity, an empty string, other prefixes.
    assert!(events(b"\x1b_far2l#host\x07").is_empty());
    assert!(events(b"\x1b_far2l\x07").is_empty());
    assert!(events(b"\x1b_far2l_marker\x07").is_empty());
    assert!(events(b"\x1b_something\x07").is_empty());
    // An event without a stack, with an unknown code, or with a stack that is too short.
    assert!(events(b"\x1b_f2l\x07").is_empty());
    assert!(events(b"\x1b_f2lAQJa\x07").is_empty());
    assert!(events(b"\x1b_f2lSw==\x07").is_empty());
    assert!(events(b"\x1b_f2l:\x07").is_empty());
}

#[test]
fn far2l_events_among_other_input() {
    let evs = events(b"a\x1b_f2lQQAAYQBD\x07\x1b[Ab");
    let sigs: Vec<Sig> = evs.iter().map(sig).collect();
    assert_eq!(
        sigs,
        vec![
            (0x41, 'a', 0, true),
            (0x41, 'a', 0, true),
            (0x26, '\0', ENH, true),
            (0x42, 'b', 0, true),
        ]
    );
}

#[test]
fn far2l_string_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b_f2lAQBB", &mut out);
    assert!(out.is_empty());
    d.feed(b"AB4AAAAAAGEAAABL", &mut out);
    assert!(out.is_empty());
    d.feed(b"\x1b", &mut out);
    assert!(out.is_empty());
    d.feed(b"\\", &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(sig(&out[0]), (0x41, 'a', 0, true));
    assert!(!d.has_pending());
}
