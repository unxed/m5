//! Tests of win32-input-mode. The field order and defaults are those of the win32-input-mode
//! description of Windows Terminal (`CSI Vk ; Sc ; Uc ; Kd ; Cs ; Rc _`).

use super::*;
use crate::decode::win32::{DISABLE, ENABLE, encode};

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

fn first(bytes: &[u8]) -> InputEvent {
    events(bytes).remove(0)
}

#[test]
fn letter_press_and_release() {
    let down = first(b"\x1b[65;30;97;1;0;1_");
    assert_eq!(sig(&down), (65, 'a', 0, true));
    assert_eq!(down.virtual_scan_code, 30);
    assert_eq!(down.repeat_count, 1);
    assert_eq!(down.input_source, "win32");
    assert!(!down.is_legacy);
    let up = first(b"\x1b[65;30;97;0;0;1_");
    assert_eq!(sig(&up), (65, 'a', 0, false));
}

#[test]
fn modifiers_and_enhanced_keys() {
    assert_eq!(one(b"\x1b[65;30;1;1;8;1_"), (65, '\u{1}', CTRL, true));
    assert_eq!(one(b"\x1b[37;75;0;1;256;1_"), (0x25, '\0', ENH, true));
    assert_eq!(one(b"\x1b[65;30;65;1;16;1_"), (65, 'A', SHIFT, true));
    assert_eq!(one(b"\x1b[13;28;13;1;0;1_"), (13, '\r', 0, true));
    assert_eq!(one(b"\x1b[16;42;0;1;16;1_"), (0x10, '\0', SHIFT, true));
}

#[test]
fn omitted_fields_take_their_defaults() {
    let ev = first(b"\x1b[65_");
    assert_eq!(sig(&ev), (65, '\0', 0, false));
    assert_eq!(ev.virtual_scan_code, 0);
    assert_eq!(ev.repeat_count, 1);
    let ev = first(b"\x1b[;;97;1_");
    assert_eq!(sig(&ev), (0, 'a', 0, true));
    assert_eq!(ev.repeat_count, 1);
    assert_eq!(events(b"\x1b[65;30;97;1;0;0_")[0].repeat_count, 1);
}

#[test]
fn repeat_count() {
    assert_eq!(events(b"\x1b[65;30;97;1;0;3_")[0].repeat_count, 3);
    assert_eq!(events(b"\x1b[65;30;97;1;0;70000_")[0].repeat_count, 0xFFFF);
}

#[test]
fn surrogate_pairs_become_one_character() {
    // U+1F600 is the UTF-16 pair D83D DE00 = 55357 56832.
    let evs = events(b"\x1b[0;0;55357;1;0;1_\x1b[0;0;56832;1;0;1_");
    assert_eq!(evs.len(), 1);
    assert_eq!(sig(&evs[0]), (0, '😀', 0, true));
    let up = events(b"\x1b[0;0;55357;0;0;1_\x1b[0;0;56832;0;0;1_");
    assert_eq!(up.len(), 1);
    assert_eq!(sig(&up[0]), (0, '😀', 0, false));
}

#[test]
fn surrogate_pair_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[0;0;55357;1;0;1_", &mut out);
    assert!(out.is_empty());
    d.feed(b"\x1b[0;0;568", &mut out);
    assert!(out.is_empty());
    d.feed(b"32;1;0;1_", &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].char_code, '😀');
}

#[test]
fn broken_surrogates_are_dropped() {
    // A lone low surrogate.
    assert!(events(b"\x1b[0;0;56832;1;0;1_").is_empty());
    // A high surrogate followed by an ordinary key: only the key remains.
    let evs = events(b"\x1b[0;0;55357;1;0;1_\x1b[65;30;97;1;0;1_");
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].char_code, 'a');
    // The stale half does not pair up later.
    let evs = events(b"\x1b[0;0;55357;1;0;1_\x1b[65;30;97;1;0;1_\x1b[0;0;56832;1;0;1_");
    assert_eq!(evs.len(), 1);
}

#[test]
fn private_and_unrelated_sequences_are_not_keys() {
    // The reply to the mode query is `CSI ? 9001 ; 1 $ y`; it is not input.
    assert!(events(b"\x1b[?9001;1$y").is_empty());
    assert!(events(b"\x1b[?9001h").is_empty());
}

#[test]
fn mixed_with_other_input() {
    let evs = events(b"x\x1b[65;30;97;1;0;1_\x1b[A");
    let sigs: Vec<Sig> = evs.iter().map(sig).collect();
    assert_eq!(
        sigs,
        vec![
            (0x58, 'x', 0, true),
            (65, 'a', 0, true),
            (0x26, '\0', ENH, true)
        ]
    );
}

#[test]
fn encode_key_events() {
    let ev = first(b"\x1b[65;30;97;1;0;1_");
    assert_eq!(encode(&ev), Some(b"\x1b[65;30;97;1;0;1_".to_vec()));
    let up = first(b"\x1b[37;75;0;0;256;2_");
    assert_eq!(encode(&up), Some(b"\x1b[37;75;0;0;256;2_".to_vec()));
    // A legacy event has no scan code and gets a repeat count of 1.
    let mut legacy = Vec::new();
    Decoder::new().feed(b"a", &mut legacy);
    assert_eq!(encode(&legacy[0]), Some(b"\x1b[65;0;97;1;0;1_".to_vec()));
}

#[test]
fn encode_character_outside_the_bmp_as_two_sequences() {
    let ev = InputEvent::key(0, 0, '😀', true);
    assert_eq!(
        encode(&ev),
        Some(b"\x1b[0;0;55357;1;0;1_\x1b[0;0;56832;1;0;1_".to_vec())
    );
}

#[test]
fn encode_refuses_other_events_and_round_trips_pairs() {
    assert_eq!(encode(&InputEvent::focus(true)), None);
    assert_eq!(encode(&InputEvent::mouse(0, 0, 0, 0)), None);
    let seq = b"\x1b[0;0;55357;1;0;1_\x1b[0;0;56832;1;0;1_";
    assert_eq!(encode(&events(seq)[0]).as_deref(), Some(&seq[..]));
}

#[test]
fn mode_switch_sequences() {
    assert_eq!(ENABLE, b"\x1b[?9001h");
    assert_eq!(DISABLE, b"\x1b[?9001l");
}
