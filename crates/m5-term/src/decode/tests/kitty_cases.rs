//! Tests of the kitty keyboard protocol. Vectors marked "spec" are literal examples of
//! <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>; the rest follow its field rules
//! and its table of functional keys.

use super::*;
use crate::decode::kitty::{
    ALL_KEYS, ALTERNATE_KEYS, ASSOCIATED_TEXT, DISAMBIGUATE, EVENT_TYPES, POP, QUERY, enable,
    encode,
};

const CAPS: u32 = 0x0080;
const NUM: u32 = 0x0020;
const RCTRL: u32 = 0x0004;
const RALT: u32 = 0x0001;

fn events(bytes: &[u8]) -> Vec<InputEvent> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(bytes, &mut out);
    d.flush_timeout(&mut out);
    out
}

fn first(bytes: &[u8]) -> InputEvent {
    let mut all = events(bytes);
    assert!(!all.is_empty(), "{bytes:?} gave no event");
    all.remove(0)
}

fn enc(bytes: &[u8], flags: u8) -> Option<Vec<u8>> {
    encode(&first(bytes), flags)
}

const FULL: u8 = DISAMBIGUATE | ALL_KEYS | ASSOCIATED_TEXT;

#[test]
fn text_keys() {
    // spec: shift+a -> CSI 97 ; 2 ; 65 u
    assert_eq!(one(b"\x1b[97;2;65u"), (0x41, 'A', SHIFT, true));
    assert_eq!(first(b"\x1b[97;2;65u").unshifted_char, 'a');
    assert_eq!(one(b"\x1b[97u"), (0x41, 'a', 0, true));
    assert_eq!(one(b"\x1b[97;;97u"), (0x41, 'a', 0, true));
    assert_eq!(one(b"\x1b[97;5u"), (0x41, '\u{1}', CTRL, true));
    assert_eq!(one(b"\x1b[97;3u"), (0x41, 'a', ALT, true));
    assert_eq!(one(b"\x1b[97;7u"), (0x41, '\u{1}', CTRL | ALT, true));
    // Super (bit 8) is dropped.
    assert_eq!(
        one(b"\x1b[97;16u"),
        (0x41, '\u{1}', SHIFT | ALT | CTRL, true)
    );
    assert_eq!(one(b"\x1b[49;2u"), (0x31, '1', SHIFT, true));
    assert_eq!(one(b"\x1b[59;5u"), (0xBA, ';', CTRL, true));
    assert_eq!(one(b"\x1b[32u"), (0x20, ' ', 0, true));
    // Upper case key codes (xterm formatOtherKeys=1) are the same key.
    assert_eq!(one(b"\x1b[65;2u"), (0x41, 'A', SHIFT, true));
}

#[test]
fn event_types() {
    let up = first(b"\x1b[97;1:3u");
    assert!(!up.key_down);
    assert_eq!(up.virtual_key_code, 0x41);
    assert_eq!(up.control_key_state.0, 0);
    assert!(!up.is_legacy);
    assert_eq!(up.input_source, "kitty");
    assert_eq!(up.repeat_count, 1);
    assert_eq!(one(b"\x1b[97;1:2u"), (0x41, 'a', 0, true));
    assert_eq!(one(b"\x1b[97;1:1u"), (0x41, 'a', 0, true));
    assert_eq!(one(b"\x1b[27;1:3u"), (0x1B, '\u{1b}', 0, false));
    assert_eq!(one(b"\x1b[97;5:3u"), (0x41, '\u{1}', CTRL, false));
}

#[test]
fn lock_modifiers() {
    assert_eq!(one(b"\x1b[97;65u"), (0x41, 'A', CAPS, true));
    assert_eq!(one(b"\x1b[57400;129u"), (0x61, '1', NUM, true));
    assert_eq!(one(b"\x1b[97;66u"), (0x41, 'A', SHIFT | CAPS, true));
}

#[test]
fn associated_text() {
    // spec: alt+a on a layout that composes it -> CSI 0 ; ; 229 u
    assert_eq!(one(b"\x1b[0;;229u"), (0, 'å', 0, true));
    assert_eq!(
        decode(b"\x1b[0;;104:105u"),
        vec![(0, 'h', 0, true), (0x49, 'i', 0, true)]
    );
    assert_eq!(one(b"\x1b[97;2;1071u"), (0x41, 'Я', SHIFT, true));
}

#[test]
fn text_of_ctrl_combinations_is_ignored() {
    // Bytes captured from WezTerm (nightly) with enable_kitty_keyboard and `CSI > 31 u`
    // in the key-test CI job: the plain letter comes as text although Ctrl is held.
    assert_eq!(one(b"\x1b[97;5;97u"), (0x41, '\u{1}', CTRL, true));
    assert_eq!(one(b"\x1b[122;5;122u"), (0x5A, '\u{1a}', CTRL, true));
    assert_eq!(one(b"\x1b[97;7;97u"), (0x41, '\u{1}', CTRL | ALT, true));
    assert_eq!(one(b"\x1b[121:89;6;89u"), (0x59, '\u{19}', CTRL | SHIFT, true));
    // The same events without the text give the same result.
    for (with_text, without) in [
        (&b"\x1b[91;5;91u"[..], &b"\x1b[91;5u"[..]),
        (&b"\x1b[47;5;47u"[..], &b"\x1b[47;5u"[..]),
        (&b"\x1b[97;7;97u"[..], &b"\x1b[97;7u"[..]),
    ] {
        assert_eq!(one(with_text), one(without));
    }
    // Text that is not the plain letter is still used.
    assert_eq!(one(b"\x1b[97;5;1103u"), (0x41, '\u{44f}', CTRL, true));
}

#[test]
fn alternate_keys() {
    assert_eq!(one(b"\x1b[49:33;2u"), (0x31, '!', SHIFT, true));
    // Ctrl+C typed on a Russian layout: key 1089, shifted 1057, base layout key 99 ('c').
    assert_eq!(one(b"\x1b[1089:1057:99;5u"), (0x43, '\u{3}', CTRL, true));
    assert_eq!(one("\u{1b}[1103;2u".as_bytes()), (0, 'Я', SHIFT, true));
    assert_eq!(one("\u{1b}[1103::113u".as_bytes()), (0x51, 'я', 0, true));
}

#[test]
fn functional_keys_of_the_csi_u_family() {
    assert_eq!(one(b"\x1b[27u"), (0x1B, '\u{1b}', 0, true));
    assert_eq!(one(b"\x1b[13u"), (0x0D, '\r', 0, true));
    assert_eq!(one(b"\x1b[9;2u"), (0x09, '\t', SHIFT, true));
    assert_eq!(one(b"\x1b[127u"), (0x08, '\u{8}', 0, true));
    assert_eq!(one(b"\x1b[13;5u"), (0x0D, '\r', CTRL, true));
}

#[test]
fn legacy_forms_with_event_types() {
    let up = first(b"\x1b[1;5:3A");
    assert_eq!(sig(&up), (0x26, '\0', CTRL | ENH, false));
    assert!(!up.is_legacy);
    assert_eq!(one(b"\x1b[3;1:3~"), (0x2E, '\0', ENH, false));
    assert_eq!(one(b"\x1b[1;1:2P"), (0x70, '\0', 0, true));
    // F3 is CSI 13 ~ in the kitty protocol.
    assert_eq!(one(b"\x1b[13;2~"), (0x72, '\0', SHIFT, true));
    assert_eq!(one(b"\x1b[1;2:1H"), (0x24, '\0', SHIFT | ENH, true));
}

#[test]
fn f13_and_up() {
    assert_eq!(one(b"\x1b[57376u"), (0x7C, '\0', 0, true));
    assert_eq!(one(b"\x1b[57387u"), (0x87, '\0', 0, true));
    // F25 and above have no virtual key.
    assert!(decode(b"\x1b[57388u").is_empty());
    assert!(decode(b"\x1b[57398u").is_empty());
}

#[test]
fn keypad_keys() {
    assert_eq!(one(b"\x1b[57399u"), (0x60, '0', 0, true));
    assert_eq!(one(b"\x1b[57408u"), (0x69, '9', 0, true));
    assert_eq!(one(b"\x1b[57409u"), (0x6E, '.', 0, true));
    assert_eq!(one(b"\x1b[57410u"), (0x6F, '/', ENH, true));
    assert_eq!(one(b"\x1b[57411u"), (0x6A, '*', 0, true));
    assert_eq!(one(b"\x1b[57412u"), (0x6D, '-', 0, true));
    assert_eq!(one(b"\x1b[57413u"), (0x6B, '+', 0, true));
    assert_eq!(one(b"\x1b[57414u"), (0x0D, '\r', ENH, true));
    assert_eq!(one(b"\x1b[57416u"), (0x6C, ',', 0, true));
    // Navigation keys of the keypad (Num Lock off) have no ENHANCED_KEY.
    assert_eq!(one(b"\x1b[57417u"), (0x25, '\0', 0, true));
    assert_eq!(one(b"\x1b[57418u"), (0x27, '\0', 0, true));
    assert_eq!(one(b"\x1b[57419u"), (0x26, '\0', 0, true));
    assert_eq!(one(b"\x1b[57420u"), (0x28, '\0', 0, true));
    assert_eq!(one(b"\x1b[57421u"), (0x21, '\0', 0, true));
    assert_eq!(one(b"\x1b[57422u"), (0x22, '\0', 0, true));
    assert_eq!(one(b"\x1b[57423u"), (0x24, '\0', 0, true));
    assert_eq!(one(b"\x1b[57424u"), (0x23, '\0', 0, true));
    assert_eq!(one(b"\x1b[57425u"), (0x2D, '\0', 0, true));
    assert_eq!(one(b"\x1b[57426u"), (0x2E, '\0', 0, true));
    assert_eq!(one(b"\x1b[57427u"), (0x0C, '\0', 0, true));
}

#[test]
fn lock_and_system_keys() {
    assert_eq!(one(b"\x1b[57358u"), (0x14, '\0', 0, true));
    assert_eq!(one(b"\x1b[57359u"), (0x91, '\0', 0, true));
    assert_eq!(one(b"\x1b[57360u"), (0x90, '\0', 0, true));
    assert_eq!(one(b"\x1b[57361u"), (0x2C, '\0', ENH, true));
    assert_eq!(one(b"\x1b[57362u"), (0x13, '\0', 0, true));
    assert_eq!(one(b"\x1b[57363u"), (0x5D, '\0', ENH, true));
}

#[test]
fn modifier_keys() {
    let shift = first(b"\x1b[57441;2u");
    assert_eq!(sig(&shift), (0x10, '\0', SHIFT, true));
    assert_eq!(shift.virtual_scan_code, 0x2A);
    let shift_up = first(b"\x1b[57441;1:3u");
    assert_eq!(sig(&shift_up), (0x10, '\0', 0, false));
    let right = first(b"\x1b[57447;2u");
    assert_eq!(sig(&right), (0x10, '\0', SHIFT, true));
    assert_eq!(right.virtual_scan_code, 0x36);
    assert_eq!(one(b"\x1b[57442;5u"), (0x11, '\0', CTRL, true));
    assert_eq!(one(b"\x1b[57448;5u"), (0x11, '\0', RCTRL | ENH, true));
    assert_eq!(one(b"\x1b[57443;3u"), (0x12, '\0', ALT, true));
    assert_eq!(one(b"\x1b[57449;3u"), (0x12, '\0', RALT | ENH, true));
    assert_eq!(one(b"\x1b[57453;3u"), (0x12, '\0', RALT | ENH, true));
    assert_eq!(one(b"\x1b[57444;9u"), (0x5B, '\0', 0, true));
    assert_eq!(one(b"\x1b[57450;9u"), (0x5C, '\0', 0, true));
    // Ctrl pressed while Shift is held keeps Shift.
    assert_eq!(one(b"\x1b[57442;6u"), (0x11, '\0', SHIFT | CTRL, true));
}

#[test]
fn what_is_not_a_key_event_is_dropped() {
    // Query reply, push, pop and a bare `CSI u` (restore cursor).
    assert!(decode(b"\x1b[?0u").is_empty());
    assert!(decode(b"\x1b[>1u").is_empty());
    assert!(decode(b"\x1b[<u").is_empty());
    assert!(decode(b"\x1b[u").is_empty());
    // Media keys, Hyper and Meta have no virtual key; control codes are no keys.
    assert!(decode(b"\x1b[57428u").is_empty());
    assert!(decode(b"\x1b[57445u").is_empty());
    assert!(decode(b"\x1b[57446u").is_empty());
    assert!(decode(b"\x1b[1u").is_empty());
}

#[test]
fn kitty_sequence_split_across_feeds() {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    d.feed(b"\x1b[97;", &mut out);
    assert!(out.is_empty());
    d.feed(b"2;65u", &mut out);
    assert_eq!(
        out.iter().map(sig).collect::<Vec<_>>(),
        [(0x41, 'A', SHIFT, true)]
    );
}

#[test]
fn encode_spec_examples() {
    // spec: shift+a -> CSI 97 ; 2 ; 65 u
    assert_eq!(enc(b"\x1b[97;2;65u", FULL), Some(b"\x1b[97;2;65u".to_vec()));
    assert_eq!(enc(b"A", FULL), Some(b"\x1b[97;2;65u".to_vec()));
    // Without the text flag the text field is left out.
    assert_eq!(
        enc(b"A", DISAMBIGUATE | ALL_KEYS),
        Some(b"\x1b[97;2u".to_vec())
    );
}

#[test]
fn encode_text_keys_by_flags() {
    assert_eq!(enc(b"a", DISAMBIGUATE), Some(b"a".to_vec()));
    assert_eq!(enc(b"A", DISAMBIGUATE), Some(b"A".to_vec()));
    assert_eq!(
        enc("я".as_bytes(), DISAMBIGUATE),
        Some("я".as_bytes().to_vec())
    );
    assert_eq!(
        enc(b"a", DISAMBIGUATE | ALL_KEYS),
        Some(b"\x1b[97u".to_vec())
    );
    assert_eq!(enc(b"a", FULL), Some(b"\x1b[97;;97u".to_vec()));
    assert_eq!(enc(&[0x01], DISAMBIGUATE), Some(b"\x1b[97;5u".to_vec()));
    assert_eq!(enc(b"\x1ba", DISAMBIGUATE), Some(b"\x1b[97;3u".to_vec()));
    assert_eq!(enc(b"\x1bA", DISAMBIGUATE), Some(b"\x1b[97;4u".to_vec()));
}

#[test]
fn encode_event_types() {
    let release = first(b"\x1b[97;1:3u");
    assert_eq!(encode(&release, DISAMBIGUATE), None);
    assert_eq!(encode(&release, DISAMBIGUATE | EVENT_TYPES), None);
    assert_eq!(
        encode(&release, DISAMBIGUATE | EVENT_TYPES | ALL_KEYS),
        Some(b"\x1b[97;1:3u".to_vec())
    );
    let up = first(b"\x1b[1;5:3A");
    assert_eq!(encode(&up, DISAMBIGUATE), None);
    assert_eq!(
        encode(&up, DISAMBIGUATE | EVENT_TYPES),
        Some(b"\x1b[1;5:3A".to_vec())
    );
}

#[test]
fn encode_enter_tab_backspace_escape() {
    assert_eq!(enc(&[0x1B], DISAMBIGUATE), Some(b"\x1b[27u".to_vec()));
    assert_eq!(enc(b"\r", DISAMBIGUATE), Some(b"\r".to_vec()));
    assert_eq!(enc(b"\t", DISAMBIGUATE), Some(b"\t".to_vec()));
    assert_eq!(enc(&[0x7F], DISAMBIGUATE), Some(vec![0x7F]));
    assert_eq!(
        enc(b"\r", DISAMBIGUATE | ALL_KEYS),
        Some(b"\x1b[13u".to_vec())
    );
    assert_eq!(
        enc(b"\x1b[13;2u", DISAMBIGUATE),
        Some(b"\x1b[13;2u".to_vec())
    );
    assert_eq!(
        enc(b"\x1b[127;5u", DISAMBIGUATE),
        Some(b"\x1b[127;5u".to_vec())
    );
    let release = first(b"\x1b[127;1:3u");
    assert_eq!(encode(&release, DISAMBIGUATE | EVENT_TYPES), None);
    assert_eq!(
        encode(&release, DISAMBIGUATE | EVENT_TYPES | ALL_KEYS),
        Some(b"\x1b[127;1:3u".to_vec())
    );
}

#[test]
fn encode_legacy_form_keys() {
    assert_eq!(enc(b"\x1b[A", DISAMBIGUATE), Some(b"\x1b[A".to_vec()));
    assert_eq!(enc(b"\x1b[1;5A", DISAMBIGUATE), Some(b"\x1b[1;5A".to_vec()));
    assert_eq!(enc(b"\x1b[H", DISAMBIGUATE), Some(b"\x1b[H".to_vec()));
    assert_eq!(enc(b"\x1b[4~", DISAMBIGUATE), Some(b"\x1b[F".to_vec()));
    assert_eq!(enc(b"\x1b[3~", DISAMBIGUATE), Some(b"\x1b[3~".to_vec()));
    assert_eq!(enc(b"\x1b[3;5~", DISAMBIGUATE), Some(b"\x1b[3;5~".to_vec()));
    assert_eq!(enc(b"\x1b[5~", DISAMBIGUATE), Some(b"\x1b[5~".to_vec()));
    assert_eq!(enc(b"\x1bOP", DISAMBIGUATE), Some(b"\x1b[P".to_vec()));
    assert_eq!(enc(b"\x1bOR", DISAMBIGUATE), Some(b"\x1b[13~".to_vec()));
    assert_eq!(enc(b"\x1b[1;2P", DISAMBIGUATE), Some(b"\x1b[1;2P".to_vec()));
    assert_eq!(enc(b"\x1b[15~", DISAMBIGUATE), Some(b"\x1b[15~".to_vec()));
    assert_eq!(enc(b"\x1b[24~", DISAMBIGUATE), Some(b"\x1b[24~".to_vec()));
    assert_eq!(
        enc(b"\x1b[25~", DISAMBIGUATE),
        Some(b"\x1b[57376u".to_vec())
    );
}

#[test]
fn encode_modifier_keys_need_all_keys() {
    assert_eq!(enc(b"\x1b[57441;2u", DISAMBIGUATE), None);
    assert_eq!(
        enc(b"\x1b[57441;2u", DISAMBIGUATE | ALL_KEYS),
        Some(b"\x1b[57441;2u".to_vec())
    );
}

#[test]
fn encode_alternate_keys() {
    let flags = DISAMBIGUATE | ALL_KEYS | ALTERNATE_KEYS;
    assert_eq!(
        enc(b"\x1b[49:33;2u", flags),
        Some(b"\x1b[49:33;2u".to_vec())
    );
    // The shifted key is only reported when it differs.
    assert_eq!(
        enc(b"\x1b[97;2;65u", flags),
        Some(b"\x1b[97:65;2u".to_vec())
    );
    // The base layout key of a Russian layout.
    assert_eq!(
        enc(b"\x1b[1089:1057:99;5u", flags),
        Some(b"\x1b[1089::99;5u".to_vec())
    );
}

#[test]
fn encode_refuses_what_it_cannot_write() {
    assert_eq!(encode(&InputEvent::focus(true), FULL), None);
    assert_eq!(encode(&InputEvent::mouse(1, 1, 0, 0), FULL), None);
    // The legacy encoding is not produced.
    assert_eq!(enc(b"a", 0), None);
    assert_eq!(enc(b"a", ALL_KEYS), None);
    // A key without a virtual key or a character.
    let blank = InputEvent::key(0, 0, '\0', true);
    assert_eq!(encode(&blank, FULL), None);
}

#[test]
fn decode_then_encode_round_trips() {
    let flags = DISAMBIGUATE | EVENT_TYPES | ALL_KEYS | ASSOCIATED_TEXT;
    let sequences: [&[u8]; 24] = [
        b"\x1b[97;;97u",
        b"\x1b[97;2;65u",
        b"\x1b[97;5u",
        b"\x1b[97;3u",
        b"\x1b[97;1:3u",
        b"\x1b[1;5A",
        b"\x1b[A",
        b"\x1b[1;1:3A",
        b"\x1b[15;2~",
        b"\x1b[3;5~",
        b"\x1b[57441;2u",
        b"\x1b[57441;1:3u",
        b"\x1b[57442;5u",
        b"\x1b[57448;5u",
        b"\x1b[57449;3u",
        b"\x1b[27u",
        b"\x1b[13u",
        b"\x1b[9;2u",
        b"\x1b[127u",
        b"\x1b[57399u",
        b"\x1b[57417u",
        b"\x1b[57414u",
        b"\x1b[57376u",
        b"\x1b[57427u",
    ];
    for seq in sequences {
        assert_eq!(enc(seq, flags).as_deref(), Some(seq), "{seq:?}");
    }
}

#[test]
fn flag_sequences() {
    assert_eq!(enable(1), b"\x1b[>1u".to_vec());
    assert_eq!(enable(31), b"\x1b[>31u".to_vec());
    assert_eq!(QUERY, b"\x1b[?u");
    assert_eq!(POP, b"\x1b[<u");
    assert_eq!(DISAMBIGUATE, 1);
    assert_eq!(EVENT_TYPES, 2);
    assert_eq!(ALTERNATE_KEYS, 4);
    assert_eq!(ALL_KEYS, 8);
    assert_eq!(ASSOCIATED_TEXT, 16);
}
