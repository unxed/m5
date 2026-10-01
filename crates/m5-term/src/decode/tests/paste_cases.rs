//! Tests of bracketed paste (xterm ctlseqs, "Bracketed Paste Mode": `CSI 200 ~` starts and
//! `CSI 201 ~` ends the pasted text).

use super::*;
use crate::decode::paste::{DISABLE, ENABLE};

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

/// Renders events compactly: `<` and `>` for the markers, the character for keys.
fn render(evs: &[InputEvent]) -> String {
    let mut text = String::new();
    for ev in evs {
        match ev.event_type {
            EventType::Paste if ev.paste_start => text.push('<'),
            EventType::Paste => text.push('>'),
            EventType::Key => text.push(ev.char_code),
            _ => text.push('?'),
        }
    }
    text
}

#[test]
fn text_between_the_markers() {
    let evs = events(b"\x1b[200~hello\x1b[201~");
    assert_eq!(render(&evs), "<hello>");
    assert!(evs[0].paste_start);
    assert!(!evs[6].paste_start);
    assert_eq!(evs[0].input_source, "bracketed_paste");
    let h = sig(&evs[1]);
    assert_eq!(h, (0x48, 'h', 0, true));
    assert!(evs[1].is_legacy);
}

#[test]
fn empty_paste() {
    assert_eq!(render(&events(b"\x1b[200~\x1b[201~")), "<>");
}

#[test]
fn escape_sequences_in_the_text_are_not_interpreted() {
    let evs = events(b"\x1b[200~\x1b[A\x1b[201~");
    assert_eq!(render(&evs), "<\u{1b}[A>");
    assert_eq!(sig(&evs[1]), (0, '\u{1b}', 0, true));
}

#[test]
fn control_characters_are_text_not_keys() {
    let evs = events(b"\x1b[200~a\r\nb\tc\x01\x7f\x1b[201~");
    assert_eq!(render(&evs), "<a\r\nb\tc\u{1}\u{7f}>");
    assert_eq!(sig(&evs[2]), (0x0D, '\r', 0, true));
    assert_eq!(sig(&evs[3]), (0x0D, '\n', 0, true));
    assert_eq!(sig(&evs[5]), (0x09, '\t', 0, true));
    // Ctrl+A is not what was pasted.
    assert_eq!(sig(&evs[7]), (0, '\u{1}', 0, true));
    assert_eq!(sig(&evs[8]), (0, '\u{7f}', 0, true));
}

#[test]
fn utf8_text() {
    let evs = events("\u{1b}[200~я😀\u{1b}[201~".as_bytes());
    assert_eq!(render(&evs), "<я😀>");
    assert_eq!(sig(&evs[1]), (0, 'я', 0, true));
    // Invalid bytes become replacement characters.
    let evs = events(b"\x1b[200~\xff\x1b[201~");
    assert_eq!(render(&evs), "<\u{FFFD}>");
}

#[test]
fn keys_resume_after_the_paste() {
    let evs = events(b"a\x1b[200~x\x1b[201~\x1b[Ab");
    assert_eq!(render(&evs), "a<x>\0b");
    assert_eq!(sig(&evs[4]), (0x26, '\0', ENH, true));
}

#[test]
fn end_marker_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[200~ab\x1b[2", &mut out);
    assert_eq!(render(&out), "<ab");
    assert!(d.has_pending());
    d.feed(b"01~z", &mut out);
    assert_eq!(render(&out), "<ab>z");
    assert!(!d.has_pending());
}

#[test]
fn start_marker_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[20", &mut out);
    assert!(out.is_empty());
    d.feed(b"0~q", &mut out);
    assert_eq!(render(&out), "<q");
}

#[test]
fn escape_that_is_not_the_end_marker_is_text() {
    let evs = events(b"\x1b[200~\x1b[20x\x1b[201~");
    assert_eq!(render(&evs), "<\u{1b}[20x>");
}

#[test]
fn timeout_inside_a_paste_turns_a_cut_marker_into_text() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[200~x\x1b[20", &mut out);
    d.flush_timeout(&mut out);
    assert_eq!(render(&out), "<x\u{1b}[20");
    assert!(!d.has_pending());
    // Still in paste mode: the real end marker closes it.
    d.feed(b"\x1b[201~", &mut out);
    assert_eq!(render(&out), "<x\u{1b}[20>");
    // A lone Esc inside a paste is text too.
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[200~\x1b", &mut out);
    d.flush_timeout(&mut out);
    assert_eq!(render(&out), "<\u{1b}");
}

#[test]
fn stray_end_marker_and_other_200s() {
    assert_eq!(render(&events(b"\x1b[201~")), ">");
    // Not the marker: a modified or longer parameter list.
    assert!(events(b"\x1b[200;5~").is_empty());
}

#[test]
fn large_paste() {
    let mut input = b"\x1b[200~".to_vec();
    input.extend(std::iter::repeat_n(b'x', 100_000));
    input.extend_from_slice(b"\x1b[201~");
    let evs = events(&input);
    assert_eq!(evs.len(), 100_002);
    assert!(!evs[100_001].paste_start);
}

#[test]
fn mode_switch_sequences() {
    assert_eq!(ENABLE, b"\x1b[?2004h");
    assert_eq!(DISABLE, b"\x1b[?2004l");
}
