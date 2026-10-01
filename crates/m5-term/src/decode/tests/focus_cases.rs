//! Tests of focus reports (xterm ctlseqs, "Focus Event Tracking": `CSI I` in, `CSI O` out).

use super::*;
use crate::decode::focus::{DISABLE, ENABLE};

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

#[test]
fn focus_in_and_out() {
    let evs = events(b"\x1b[I\x1b[O");
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0].event_type, EventType::Focus);
    assert!(evs[0].set_focus);
    assert_eq!(evs[1].event_type, EventType::Focus);
    assert!(!evs[1].set_focus);
    assert_eq!(evs[0].input_source, "focus");
}

#[test]
fn focus_among_keys() {
    let evs = events(b"a\x1b[Ob\x1b[I");
    assert_eq!(evs.len(), 4);
    assert_eq!(evs[0].char_code, 'a');
    assert!(!evs[1].set_focus);
    assert_eq!(evs[2].char_code, 'b');
    assert!(evs[3].set_focus);
}

#[test]
fn focus_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[", &mut out);
    assert!(out.is_empty());
    d.feed(b"O", &mut out);
    assert_eq!(out.len(), 1);
    assert!(!out[0].set_focus);
    assert!(!d.has_pending());
}

#[test]
fn other_sequences_with_the_same_final_byte_are_not_focus() {
    assert!(events(b"\x1b[1I").is_empty());
    assert!(events(b"\x1b[;I").is_empty());
    assert!(events(b"\x1b[?I").is_empty());
    assert!(events(b"\x1b[ I").is_empty());
}

#[test]
fn focus_is_not_decoded_inside_a_paste() {
    let evs = events(b"\x1b[200~\x1b[I\x1b[201~");
    assert!(evs.iter().all(|e| e.event_type != EventType::Focus));
}

#[test]
fn alt_prefix_does_not_hide_ss3_keys() {
    // `ESC O` is SS3, never a focus report.
    assert_eq!(sig(&events(b"\x1bOP")[0]), (0x70, '\0', 0, true));
}

#[test]
fn mode_switch_sequences() {
    assert_eq!(ENABLE, b"\x1b[?1004h");
    assert_eq!(DISABLE, b"\x1b[?1004l");
}
