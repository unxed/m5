//! Tests of the legacy (xterm-style) decoding. Vectors come from the xterm control
//! sequences document (ctlseqs) unless a comment says otherwise.

use super::csi::{Scan, StrScan, scan, scan_string};
use super::keys::{char_event, decode_char, key_with_mods, xterm_mods};
use super::*;
use crate::key::EventType;

const SHIFT: u32 = 0x0010;
const ALT: u32 = 0x0002;
const CTRL: u32 = 0x0008;
const ENH: u32 = 0x0100;

/// What a test compares: virtual key, character, control key state, key-down.
type Sig = (u16, char, u32, bool);

fn sig(ev: &InputEvent) -> Sig {
    assert_eq!(ev.event_type, EventType::Key);
    (
        ev.virtual_key_code,
        ev.char_code,
        ev.control_key_state.0,
        ev.key_down,
    )
}

/// Decodes `bytes` in one piece, then lets the escape timeout expire.
fn decode(bytes: &[u8]) -> Vec<Sig> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    assert!(!d.has_pending());
    out.iter().map(sig).collect()
}

fn one(bytes: &[u8]) -> Sig {
    let events = decode(bytes);
    assert_eq!(events.len(), 1, "{bytes:?} gave {events:?}");
    events[0]
}

#[test]
fn printable_ascii() {
    assert_eq!(one(b"a"), (0x41, 'a', 0, true));
    assert_eq!(one(b"z"), (0x5A, 'z', 0, true));
    assert_eq!(one(b"A"), (0x41, 'A', SHIFT, true));
    assert_eq!(one(b"7"), (0x37, '7', 0, true));
    assert_eq!(one(b" "), (0x20, ' ', 0, true));
    assert_eq!(one(b"-"), (0xBD, '-', 0, true));
    assert_eq!(one(b"_"), (0xBD, '_', SHIFT, true));
    assert_eq!(one(b"!"), (0x31, '!', SHIFT, true));
    assert_eq!(one(b"("), (0x39, '(', SHIFT, true));
    assert_eq!(one(b"/"), (0xBF, '/', 0, true));
    assert_eq!(one(b"?"), (0xBF, '?', SHIFT, true));
    assert_eq!(one(b"["), (0xDB, '[', 0, true));
    assert_eq!(one(b"~"), (0xC0, '~', SHIFT, true));
}

#[test]
fn control_bytes() {
    assert_eq!(one(&[0x01]), (0x41, '\u{1}', CTRL, true));
    assert_eq!(one(&[0x03]), (0x43, '\u{3}', CTRL, true));
    assert_eq!(one(&[0x1A]), (0x5A, '\u{1a}', CTRL, true));
    assert_eq!(one(&[0x00]), (0x20, ' ', CTRL, true));
    assert_eq!(one(&[0x0D]), (0x0D, '\r', 0, true));
    assert_eq!(one(&[0x09]), (0x09, '\t', 0, true));
    assert_eq!(one(&[0x0A]), (0x4A, '\n', CTRL, true));
    assert_eq!(one(&[0x08]), (0x08, '\u{8}', 0, true));
    assert_eq!(one(&[0x7F]), (0x08, '\u{8}', 0, true));
    assert_eq!(one(&[0x1C]), (0xDC, '\u{1c}', CTRL, true));
    assert_eq!(one(&[0x1D]), (0xDD, '\u{1d}', CTRL, true));
    assert_eq!(one(&[0x1E]), (0x36, '\u{1e}', CTRL, true));
    assert_eq!(one(&[0x1F]), (0xBD, '\u{1f}', CTRL, true));
}

#[test]
fn events_are_marked_legacy_key_down() {
    let mut out = Vec::new();
    Decoder::new().feed(b"x", &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].is_legacy);
    assert!(out[0].key_down);
    assert_eq!(out[0].repeat_count, 1);
    assert_eq!(out[0].input_source, "legacy_char");
}

#[test]
fn utf8_text() {
    assert_eq!(one("é".as_bytes()), (0, 'é', 0, true));
    assert_eq!(one("я".as_bytes()), (0, 'я', 0, true));
    assert_eq!(one("€".as_bytes()), (0, '€', 0, true));
    assert_eq!(one("😀".as_bytes()), (0, '😀', 0, true));
}

#[test]
fn utf8_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(&[0xF0, 0x9F], &mut out);
    assert!(out.is_empty());
    assert!(d.has_pending());
    d.feed(&[0x98], &mut out);
    assert!(out.is_empty());
    d.feed(&[0x80, b'x'], &mut out);
    let sigs: Vec<Sig> = out.iter().map(sig).collect();
    assert_eq!(sigs, vec![(0, '😀', 0, true), (0x58, 'x', 0, true)]);
    assert!(!d.has_pending());
}

#[test]
fn invalid_utf8_gives_replacement_characters() {
    assert_eq!(one(&[0xFF]), (0, '\u{FFFD}', 0, true));
    assert_eq!(one(&[0x80]), (0, '\u{FFFD}', 0, true));
    assert_eq!(
        decode(&[0xC3, 0x28]),
        vec![(0, '\u{FFFD}', 0, true), (0x39, '(', SHIFT, true)]
    );
    // Cut short by the end of the input.
    assert_eq!(decode(&[0xE2, 0x82]), vec![(0, '\u{FFFD}', 0, true)]);
}

#[test]
fn decode_char_cases() {
    assert!(matches!(decode_char(b"a"), Utf8::Char('a', 1)));
    assert!(matches!(decode_char("я".as_bytes()), Utf8::Char('я', 2)));
    assert!(matches!(decode_char(&[0xC3]), Utf8::Incomplete));
    assert!(matches!(decode_char(&[0xC3, 0x41]), Utf8::Invalid));
    assert!(matches!(decode_char(&[0xC0, 0x80]), Utf8::Invalid));
    assert!(matches!(decode_char(&[0xED, 0xA0, 0x80]), Utf8::Invalid));
}

#[test]
fn lone_escape_waits_for_the_timeout() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(&[0x1B], &mut out);
    assert!(out.is_empty());
    assert!(d.has_pending());
    d.flush_timeout(&mut out);
    assert_eq!(
        out.iter().map(sig).collect::<Vec<_>>(),
        [(0x1B, '\u{1b}', 0, true)]
    );
    assert!(!d.has_pending());
}

#[test]
fn alt_with_a_character() {
    assert_eq!(one(b"\x1ba"), (0x41, 'a', ALT, true));
    assert_eq!(one(b"\x1bA"), (0x41, 'A', ALT | SHIFT, true));
    assert_eq!(one(b"\x1b1"), (0x31, '1', ALT, true));
    assert_eq!(one(b"\x1b."), (0xBE, '.', ALT, true));
    assert_eq!(one(b"\x1b\r"), (0x0D, '\r', ALT, true));
    assert_eq!(one(b"\x1b\x7f"), (0x08, '\u{8}', ALT, true));
    assert_eq!(one(b"\x1b\x01"), (0x41, '\u{1}', ALT | CTRL, true));
    assert_eq!(one("\u{1b}я".as_bytes()), (0, 'я', ALT, true));
}

#[test]
fn alt_in_front_of_a_cut_character() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(&[0x1B, 0xD1], &mut out);
    assert!(out.is_empty());
    d.feed(&[0x8F], &mut out);
    assert_eq!(
        out.iter().map(sig).collect::<Vec<_>>(),
        [(0, 'я', ALT, true)]
    );
}

#[test]
fn double_escape() {
    // Two Esc presses in a row come as one read.
    assert_eq!(
        decode(b"\x1b\x1b"),
        vec![(0x1B, '\u{1b}', 0, true), (0x1B, '\u{1b}', 0, true)]
    );
    // Alt+Esc followed by another key.
    assert_eq!(
        decode(b"\x1b\x1bx"),
        vec![(0x1B, '\u{1b}', ALT, true), (0x58, 'x', 0, true)]
    );
    // Alt prefix in front of an arrow key sequence.
    assert_eq!(one(b"\x1b\x1b[A"), (0x26, '\0', ALT | ENH, true));
    assert_eq!(one(b"\x1b\x1bOP"), (0x70, '\0', ALT, true));
}

#[test]
fn flush_resolves_what_is_cut_short() {
    // ESC [ and ESC O alone are Alt+[ and Alt+O.
    assert_eq!(one(b"\x1b["), (0xDB, '[', ALT, true));
    assert_eq!(one(b"\x1bO"), (0x4F, 'O', ALT | SHIFT, true));
    // A longer unfinished sequence is given up: Esc, then its bytes as text.
    assert_eq!(
        decode(b"\x1b[1;5"),
        vec![
            (0x1B, '\u{1b}', 0, true),
            (0xDB, '[', 0, true),
            (0x31, '1', 0, true),
            (0xBA, ';', 0, true),
            (0x35, '5', 0, true),
        ]
    );
}

#[test]
fn sequence_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b", &mut out);
    d.feed(b"[", &mut out);
    d.feed(b"1;5", &mut out);
    assert!(out.is_empty());
    assert!(d.has_pending());
    d.feed(b"Cx", &mut out);
    assert_eq!(
        out.iter().map(sig).collect::<Vec<_>>(),
        [(0x27, '\0', CTRL | ENH, true), (0x58, 'x', 0, true)]
    );
    assert!(!d.has_pending());
}

#[test]
fn text_and_keys_mixed() {
    assert_eq!(
        decode(b"ab\x1b[Ac"),
        vec![
            (0x41, 'a', 0, true),
            (0x42, 'b', 0, true),
            (0x26, '\0', ENH, true),
            (0x43, 'c', 0, true),
        ]
    );
}

#[test]
fn cursor_keys_normal_and_application_mode() {
    // ctlseqs, "PC-Style Function Keys": Up/Down/Right/Left are CSI A..D, in
    // application cursor mode SS3 A..D.
    for (letter, key) in [(b'A', 0x26), (b'B', 0x28), (b'C', 0x27), (b'D', 0x25)] {
        assert_eq!(one(&[0x1B, b'[', letter]), (key, '\0', ENH, true));
        assert_eq!(one(&[0x1B, b'O', letter]), (key, '\0', ENH, true));
    }
    assert_eq!(one(b"\x1b[H"), (0x24, '\0', ENH, true));
    assert_eq!(one(b"\x1b[F"), (0x23, '\0', ENH, true));
    assert_eq!(one(b"\x1bOH"), (0x24, '\0', ENH, true));
    assert_eq!(one(b"\x1bOF"), (0x23, '\0', ENH, true));
}

#[test]
fn cursor_keys_with_modifiers() {
    // ctlseqs: modifier value 2 Shift, 3 Alt, 4 Shift+Alt, 5 Ctrl, 6 Shift+Ctrl,
    // 7 Alt+Ctrl, 8 Shift+Alt+Ctrl.
    assert_eq!(one(b"\x1b[1;2A"), (0x26, '\0', SHIFT | ENH, true));
    assert_eq!(one(b"\x1b[1;3D"), (0x25, '\0', ALT | ENH, true));
    assert_eq!(one(b"\x1b[1;4B"), (0x28, '\0', SHIFT | ALT | ENH, true));
    assert_eq!(one(b"\x1b[1;5C"), (0x27, '\0', CTRL | ENH, true));
    assert_eq!(one(b"\x1b[1;6H"), (0x24, '\0', SHIFT | CTRL | ENH, true));
    assert_eq!(one(b"\x1b[1;7F"), (0x23, '\0', ALT | CTRL | ENH, true));
    assert_eq!(
        one(b"\x1b[1;8A"),
        (0x26, '\0', SHIFT | ALT | CTRL | ENH, true)
    );
    // Meta (9) has no Win32 flag and is dropped.
    assert_eq!(one(b"\x1b[1;9A"), (0x26, '\0', ENH, true));
    // SS3 with a digit modifier.
    assert_eq!(one(b"\x1bO5C"), (0x27, '\0', CTRL | ENH, true));
}

#[test]
fn function_keys_f1_to_f4() {
    assert_eq!(one(b"\x1bOP"), (0x70, '\0', 0, true));
    assert_eq!(one(b"\x1bOQ"), (0x71, '\0', 0, true));
    assert_eq!(one(b"\x1bOR"), (0x72, '\0', 0, true));
    assert_eq!(one(b"\x1bOS"), (0x73, '\0', 0, true));
    assert_eq!(one(b"\x1b[1;2P"), (0x70, '\0', SHIFT, true));
    assert_eq!(one(b"\x1b[1;5S"), (0x73, '\0', CTRL, true));
    assert_eq!(one(b"\x1bO5P"), (0x70, '\0', CTRL, true));
    assert_eq!(one(b"\x1bO2Q"), (0x71, '\0', SHIFT, true));
    assert_eq!(one(b"\x1b[P"), (0x70, '\0', 0, true));
    assert_eq!(one(b"\x1b[1;3R"), (0x72, '\0', ALT, true));
}

#[test]
fn function_keys_f5_and_up() {
    // ctlseqs, "VT220-style Function Keys": F1..F12 are 11..15, 17..21, 23, 24 and
    // F13..F20 are 25, 26, 28, 29, 31..34, each followed by `~`.
    let table: [(u32, u16); 20] = [
        (11, 0x70),
        (12, 0x71),
        (13, 0x72),
        (14, 0x73),
        (15, 0x74),
        (17, 0x75),
        (18, 0x76),
        (19, 0x77),
        (20, 0x78),
        (21, 0x79),
        (23, 0x7A),
        (24, 0x7B),
        (25, 0x7C),
        (26, 0x7D),
        (28, 0x7E),
        (29, 0x7F),
        (31, 0x80),
        (32, 0x81),
        (33, 0x82),
        (34, 0x83),
    ];
    for (code, key) in table {
        let seq = format!("\u{1b}[{code}~");
        assert_eq!(one(seq.as_bytes()), (key, '\0', 0, true), "{seq:?}");
    }
    assert_eq!(one(b"\x1b[15;5~"), (0x74, '\0', CTRL, true));
    assert_eq!(one(b"\x1b[24;2~"), (0x7B, '\0', SHIFT, true));
    assert_eq!(one(b"\x1b[19;8~"), (0x77, '\0', SHIFT | ALT | CTRL, true));
}

#[test]
fn linux_console_function_keys() {
    for (i, letter) in (b'A'..=b'E').enumerate() {
        let key = 0x70 + i as u16;
        assert_eq!(one(&[0x1B, b'[', b'[', letter]), (key, '\0', 0, true));
    }
}

#[test]
fn editing_keys() {
    assert_eq!(one(b"\x1b[1~"), (0x24, '\0', ENH, true));
    assert_eq!(one(b"\x1b[2~"), (0x2D, '\0', ENH, true));
    assert_eq!(one(b"\x1b[3~"), (0x2E, '\0', ENH, true));
    assert_eq!(one(b"\x1b[4~"), (0x23, '\0', ENH, true));
    assert_eq!(one(b"\x1b[5~"), (0x21, '\0', ENH, true));
    assert_eq!(one(b"\x1b[6~"), (0x22, '\0', ENH, true));
    assert_eq!(one(b"\x1b[7~"), (0x24, '\0', ENH, true));
    assert_eq!(one(b"\x1b[8~"), (0x23, '\0', ENH, true));
    assert_eq!(one(b"\x1b[3;5~"), (0x2E, '\0', CTRL | ENH, true));
    assert_eq!(one(b"\x1b[5;2~"), (0x21, '\0', SHIFT | ENH, true));
}

#[test]
fn back_tab() {
    assert_eq!(one(b"\x1b[Z"), (0x09, '\t', SHIFT, true));
}

#[test]
fn application_keypad() {
    // ctlseqs, "Application keypad": SS3 M Enter, j * k + l , m - n . o / p..y 0..9.
    assert_eq!(one(b"\x1bOM"), (0x0D, '\r', ENH, true));
    assert_eq!(one(b"\x1bOj"), (0x6A, '*', 0, true));
    assert_eq!(one(b"\x1bOk"), (0x6B, '+', 0, true));
    assert_eq!(one(b"\x1bOl"), (0x6C, ',', 0, true));
    assert_eq!(one(b"\x1bOm"), (0x6D, '-', 0, true));
    assert_eq!(one(b"\x1bOn"), (0x6E, '.', 0, true));
    assert_eq!(one(b"\x1bOo"), (0x6F, '/', ENH, true));
    for (i, letter) in (b'p'..=b'y').enumerate() {
        let digit = char::from(b'0' + i as u8);
        let key = 0x60 + i as u16;
        assert_eq!(one(&[0x1B, b'O', letter]), (key, digit, 0, true));
    }
    assert_eq!(one(b"\x1bOE"), (0x0C, '\0', 0, true));
    assert_eq!(one(b"\x1b[E"), (0x0C, '\0', 0, true));
}

#[test]
fn rxvt_shifted_and_ctrl_arrows() {
    assert_eq!(one(b"\x1b[a"), (0x26, '\0', SHIFT | ENH, true));
    assert_eq!(one(b"\x1b[d"), (0x25, '\0', SHIFT | ENH, true));
    assert_eq!(one(b"\x1bOa"), (0x26, '\0', CTRL | ENH, true));
    assert_eq!(one(b"\x1bOc"), (0x27, '\0', CTRL | ENH, true));
}

#[test]
fn modify_other_keys() {
    // ctlseqs, modifyOtherKeys: CSI 27 ; modifier ; code ~.
    assert_eq!(one(b"\x1b[27;5;9~"), (0x09, '\t', CTRL, true));
    assert_eq!(one(b"\x1b[27;2;13~"), (0x0D, '\r', SHIFT, true));
    assert_eq!(one(b"\x1b[27;5;97~"), (0x41, '\u{1}', CTRL, true));
    assert_eq!(one(b"\x1b[27;3;97~"), (0x41, 'a', ALT, true));
    assert_eq!(one(b"\x1b[27;6;65~"), (0x41, '\u{1}', SHIFT | CTRL, true));
    assert_eq!(one(b"\x1b[27;5;91~"), (0xDB, '\u{1b}', CTRL, true));
}

#[test]
fn replies_that_are_not_input_are_dropped() {
    // Cursor position report, device attributes, device status report.
    assert!(decode(b"\x1b[24;80R").is_empty());
    assert!(decode(b"\x1b[?1;2c").is_empty());
    assert!(decode(b"\x1b[>0;95;0c").is_empty());
    assert!(decode(b"\x1b[0n").is_empty());
    // Unknown but well-formed sequences between text do not disturb it.
    assert_eq!(
        decode(b"x\x1b[99;99;99zy"),
        vec![(0x58, 'x', 0, true), (0x59, 'y', 0, true)]
    );
    assert_eq!(decode(b"\x1b[ qx"), vec![(0x58, 'x', 0, true)]);
}

#[test]
fn cursor_report_is_told_from_modified_f3() {
    // `CSI 1 ; 5 R` is Ctrl+F3 (and, ambiguously, a report for row 1).
    assert_eq!(one(b"\x1b[1;5R"), (0x72, '\0', CTRL, true));
    assert_eq!(one(b"\x1b[R"), (0x72, '\0', 0, true));
}

#[test]
fn string_sequences_are_consumed() {
    assert!(decode(b"\x1b_hello\x07").is_empty());
    assert!(decode(b"\x1b_hello\x1b\\").is_empty());
    assert_eq!(
        decode(b"a\x1b_x\x07b"),
        vec![(0x41, 'a', 0, true), (0x42, 'b', 0, true)]
    );
    // An unterminated string stays pending until it ends.
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b_abc", &mut out);
    assert!(out.is_empty());
    assert!(d.has_pending());
    d.feed(b"\x07z", &mut out);
    assert_eq!(
        out.iter().map(sig).collect::<Vec<_>>(),
        [(0x5A, 'z', 0, true)]
    );
}

#[test]
fn malformed_csi_is_skipped_without_eating_what_follows() {
    // A control byte inside the parameters ends the sequence before it.
    assert_eq!(
        decode(b"\x1b[1\x0dx"),
        vec![(0x0D, '\r', 0, true), (0x58, 'x', 0, true)]
    );
    // A new ESC interrupts an unfinished sequence.
    assert_eq!(one(b"\x1b[1\x1b[A"), (0x26, '\0', ENH, true));
}

#[test]
fn huge_unterminated_string_does_not_grow_without_bound() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b_", &mut out);
    let big = "x".repeat(MAX_PENDING + 10);
    d.feed(big.as_bytes(), &mut out);
    assert!(!d.has_pending() || d.buf.len() <= MAX_PENDING);
}

#[test]
fn scan_csi_parameters() {
    let Scan::Seq(seq, len) = scan(b"\x1b[1;5:3A") else {
        panic!("complete sequence expected");
    };
    assert_eq!(len, 8);
    assert_eq!(seq.final_byte, b'A');
    assert_eq!(seq.private, None);
    assert_eq!(seq.param(0), Some(1));
    assert_eq!(seq.param(1), Some(5));
    assert_eq!(seq.sub(1, 1), Some(3));
    assert_eq!(seq.param(2), None);

    let Scan::Seq(seq, _) = scan(b"\x1b[<0;;9M") else {
        panic!("complete sequence expected");
    };
    assert_eq!(seq.private, Some(b'<'));
    assert_eq!(seq.param(0), Some(0));
    assert_eq!(seq.param(1), None);
    assert_eq!(seq.param(2), Some(9));

    assert!(matches!(scan(b"\x1b[1;"), Scan::Incomplete));
    assert!(matches!(scan(b"\x1b[1\x07"), Scan::Bad(3)));
    let Scan::Seq(seq, _) = scan(b"\x1b[1 q") else {
        panic!("complete sequence expected");
    };
    assert!(seq.has_intermediate);
    // Parameter bytes after an intermediate byte are malformed.
    assert!(matches!(scan(b"\x1b[ 1q"), Scan::Bad(3)));
    // Huge numbers saturate.
    let Scan::Seq(seq, _) = scan(b"\x1b[99999999999999999999~") else {
        panic!("complete sequence expected");
    };
    assert_eq!(seq.param(0), Some(u32::MAX));
}

#[test]
fn scan_string_terminators() {
    assert!(matches!(scan_string(b"\x1b_ab\x07"), StrScan::Done(5)));
    assert!(matches!(scan_string(b"\x1b_ab\x1b\\"), StrScan::Done(6)));
    assert!(matches!(scan_string(b"\x1b_ab"), StrScan::Incomplete));
    assert!(matches!(scan_string(b"\x1b_ab\x1b"), StrScan::Incomplete));
    assert!(matches!(scan_string(b"\x1b_ab\x1b["), StrScan::Aborted(4)));
}

#[test]
fn xterm_modifier_values() {
    assert_eq!(xterm_mods(0), 0);
    assert_eq!(xterm_mods(1), 0);
    assert_eq!(xterm_mods(2), SHIFT);
    assert_eq!(xterm_mods(3), ALT);
    assert_eq!(xterm_mods(5), CTRL);
    assert_eq!(xterm_mods(8), SHIFT | ALT | CTRL);
    assert_eq!(xterm_mods(9), 0);
}

#[test]
fn key_with_mods_rules() {
    let ev = key_with_mods('a', CTRL);
    assert_eq!((ev.virtual_key_code, ev.char_code), (0x41, '\u{1}'));
    let ev = key_with_mods('a', SHIFT);
    assert_eq!((ev.char_code, ev.control_key_state.0), ('A', SHIFT));
    let ev = key_with_mods('1', CTRL);
    assert_eq!((ev.virtual_key_code, ev.char_code), (0x31, '1'));
    // Ctrl+Backspace-style bytes that already carry Ctrl are not converted twice.
    let ev = key_with_mods('\u{1}', ALT);
    assert_eq!(ev.control_key_state.0, CTRL | ALT);
    assert_eq!(char_event('x').virtual_key_code, 0x58);
}
