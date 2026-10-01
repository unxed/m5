use super::*;

const KITTY_ANSWER: &[u8] = b"\x1b[?0u";
const DA_ANSWER: &[u8] = b"\x1b[?62;c";
const FAR2L_ANSWER_ST: &[u8] = b"\x1b_far2lok\x1b\\";
const FAR2L_ANSWER_BEL: &[u8] = b"\x1b_far2lok\x07";

fn probe_of(chunks: &[&[u8]]) -> Probe {
    let mut probe = Probe::new();
    for chunk in chunks {
        probe.feed(chunk);
    }
    probe
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

#[test]
fn request_has_far2l_kitty_and_da_in_this_order() {
    let all = Options::default();
    let full = concat(&[b"\x1b_far2l1\x1b\\", b"\x1b[?u", b"\x1b[c"]);
    assert_eq!(probe_request(&all), full);
    let no_far2l = Options {
        far2l: false,
        ..all
    };
    assert_eq!(probe_request(&no_far2l), concat(&[b"\x1b[?u", b"\x1b[c"]));
}

#[test]
fn legacy_terminal_answers_only_da() {
    let probe = probe_of(&[DA_ANSWER]);
    assert!(probe.is_done());
    assert_eq!(
        probe.capabilities(),
        Capabilities {
            far2l: false,
            kitty_flags: None,
            primary_da: true,
        }
    );
}

#[test]
fn kitty_answer_carries_the_flags() {
    let probe = probe_of(&[b"\x1b[?5u", DA_ANSWER]);
    assert_eq!(probe.capabilities().kitty_flags, Some(5));
    assert!(probe.is_done());
    // No flags is an answer too.
    assert_eq!(
        probe_of(&[KITTY_ANSWER]).capabilities().kitty_flags,
        Some(0)
    );
    assert_eq!(
        probe_of(&[b"\x1b[?999u"]).capabilities().kitty_flags,
        Some(255)
    );
}

#[test]
fn far2l_answer_with_either_terminator() {
    for answer in [FAR2L_ANSWER_ST, FAR2L_ANSWER_BEL] {
        let probe = probe_of(&[answer, DA_ANSWER]);
        assert!(probe.capabilities().far2l, "{answer:?}");
        assert!(probe.is_done());
    }
}

#[test]
fn answers_in_one_chunk() {
    let all = concat(&[FAR2L_ANSWER_ST, KITTY_ANSWER, DA_ANSWER]);
    let mut probe = probe_of(&[&all]);
    let caps = probe.capabilities();
    assert!(caps.far2l && caps.primary_da);
    assert_eq!(caps.kitty_flags, Some(0));
    assert!(probe.take_input().is_empty());
}

#[test]
fn answers_split_at_every_byte() {
    let all = concat(&[FAR2L_ANSWER_ST, b"\x1b[?31u", DA_ANSWER]);
    let mut probe = Probe::new();
    for b in &all {
        probe.feed(&[*b]);
    }
    let caps = probe.capabilities();
    assert!(caps.far2l && caps.primary_da);
    assert_eq!(caps.kitty_flags, Some(31));
    assert!(probe.take_input().is_empty());
}

#[test]
fn typed_keys_are_kept_in_order() {
    let mut probe = probe_of(&[b"x", b"\x1b[A", KITTY_ANSWER, b"y", DA_ANSWER, b"z\x1b[B"]);
    assert!(probe.is_done());
    assert_eq!(probe.take_input(), b"x\x1b[Ayz\x1b[B");
}

#[test]
fn answers_to_other_requests_are_input() {
    // A cursor position report and some other terminal's APC string are not ours.
    let mut probe = probe_of(&[b"\x1b[12;40R", b"\x1b_Gi=1;OK\x1b\\", DA_ANSWER]);
    assert_eq!(probe.take_input(), b"\x1b[12;40R\x1b_Gi=1;OK\x1b\\");
    assert!(!probe.capabilities().far2l);
}

#[test]
fn bytes_after_the_end_marker_are_input() {
    let mut probe = probe_of(&[DA_ANSWER]);
    probe.feed(b"\x1b[?1u");
    // After the end marker even a kitty-looking answer is plain input.
    assert_eq!(probe.capabilities().kitty_flags, None);
    assert_eq!(probe.take_input(), b"\x1b[?1u");
}

#[test]
fn timeout_gives_up_on_a_cut_sequence() {
    let mut probe = probe_of(&[b"a", b"\x1b[?"]);
    assert!(!probe.is_done());
    probe.timeout();
    assert!(probe.is_done());
    assert!(!probe.capabilities().primary_da);
    assert_eq!(probe.take_input(), b"a\x1b[?");
    // A lone ESC and an unfinished APC string too.
    let mut probe = probe_of(&[b"\x1b"]);
    probe.timeout();
    assert_eq!(probe.take_input(), b"\x1b");
    let mut probe = probe_of(&[b"\x1b_far2l"]);
    probe.timeout();
    assert_eq!(probe.take_input(), b"\x1b_far2l");
    assert!(!probe.capabilities().far2l);
}

#[test]
fn escape_followed_by_a_character_is_input() {
    let mut probe = probe_of(&[b"\x1bq", DA_ANSWER]);
    assert_eq!(probe.take_input(), b"\x1bq");
}

#[test]
fn far2l_allowed_by_environment() {
    assert!(far2l_allowed(Some("xterm-256color"), None));
    assert!(far2l_allowed(None, None));
    assert!(far2l_allowed(Some("xterm"), Some("1")));
    assert!(!far2l_allowed(Some("screen"), None));
    assert!(!far2l_allowed(Some("screen-256color"), None));
    assert!(!far2l_allowed(Some("tmux-256color"), None));
    assert!(!far2l_allowed(Some("xterm"), Some("0")));
}

fn caps(far2l: bool, kitty: Option<u8>) -> Capabilities {
    Capabilities {
        far2l,
        kitty_flags: kitty,
        primary_da: true,
    }
}

#[test]
fn plan_legacy_terminal() {
    let modes = plan(&caps(false, None), &Options::default());
    assert_eq!(modes.keyboard, KeyboardMode::Legacy);
    let on = concat(&[b"\x1b[?2004h", b"\x1b[?1004h", mouse::ENABLE]);
    let off = concat(&[mouse::DISABLE, b"\x1b[?1004l", b"\x1b[?2004l"]);
    assert_eq!(modes.enable, on);
    assert_eq!(modes.disable, off);
}

#[test]
fn plan_kitty_terminal_enables_disambiguate_and_pops_on_exit() {
    let modes = plan(&caps(false, Some(0)), &Options::default());
    assert_eq!(modes.keyboard, KeyboardMode::Kitty);
    assert!(modes.enable.starts_with(b"\x1b[>1u"));
    assert!(modes.disable.ends_with(b"\x1b[<u"));
    // Switching the modes off goes in the reverse order of switching them on.
    assert!(modes.disable.starts_with(mouse::DISABLE));
}

#[test]
fn plan_far2l_wins_and_keeps_kitty_and_win32_off() {
    let opts = Options {
        win32: true,
        ..Options::default()
    };
    let modes = plan(&caps(true, Some(0)), &opts);
    assert_eq!(modes.keyboard, KeyboardMode::Far2l);
    assert!(!modes.enable.windows(3).any(|w| w == b"\x1b[>"));
    assert!(!modes.enable.windows(8).any(|w| w == b"\x1b[?9001h"));
    assert!(modes.disable.ends_with(b"\x1b_far2l0\x1b\\"));
}

#[test]
fn plan_win32_only_when_asked_and_not_kitty() {
    let asked = Options {
        win32: true,
        ..Options::default()
    };
    let modes = plan(&caps(false, None), &asked);
    assert_eq!(modes.keyboard, KeyboardMode::Win32);
    assert!(modes.enable.starts_with(b"\x1b[?9001h"));
    assert!(modes.disable.ends_with(b"\x1b[?9001l"));
    assert_eq!(
        plan(&caps(false, Some(1)), &asked).keyboard,
        KeyboardMode::Kitty
    );
    let plain = plan(&caps(false, None), &Options::default());
    assert_eq!(plain.keyboard, KeyboardMode::Legacy);
}

#[test]
fn plan_with_nothing_asked_for_writes_nothing() {
    let none = Options {
        far2l: false,
        win32: false,
        mouse: false,
        focus: false,
        paste: false,
    };
    let modes = plan(&caps(false, None), &none);
    assert!(modes.enable.is_empty() && modes.disable.is_empty());
}

fn scripted(chunks: &[&'static [u8]]) -> Box<Reader<'static>> {
    let mut left: Vec<&'static [u8]> = chunks.iter().rev().copied().collect();
    Box::new(move |wait| {
        assert!(!wait.is_zero());
        Ok(left.pop().map(<[u8]>::to_vec))
    })
}

#[test]
fn negotiate_with_a_kitty_terminal() {
    let mut out = Vec::new();
    let mut read = scripted(&[b"\x1b[?0u", b"\x1b[?62;c"]);
    let opts = Options::default();
    let got = negotiate(&mut out, &mut *read, &opts, Duration::from_millis(200)).unwrap();
    assert_eq!(got.caps.kitty_flags, Some(0));
    assert_eq!(got.modes.keyboard, KeyboardMode::Kitty);
    assert!(got.input.is_empty());
    assert_eq!(out, concat(&[&probe_request(&opts), &got.modes.enable]));
}

#[test]
fn negotiate_with_a_far2l_terminal_and_a_typed_key() {
    let mut out = Vec::new();
    let mut read = scripted(&[b"a\x1b_far2lok\x1b\\", b"\x1b[?62;c"]);
    let opts = Options::default();
    let got = negotiate(&mut out, &mut *read, &opts, Duration::from_millis(200)).unwrap();
    assert!(got.caps.far2l);
    assert_eq!(got.modes.keyboard, KeyboardMode::Far2l);
    assert_eq!(got.input, b"a");
}

#[test]
fn negotiate_with_a_silent_terminal_falls_back_to_legacy() {
    let mut out = Vec::new();
    let mut read = scripted(&[]);
    let opts = Options::default();
    let got = negotiate(&mut out, &mut *read, &opts, Duration::from_millis(200)).unwrap();
    assert!(!got.caps.primary_da);
    assert_eq!(got.modes.keyboard, KeyboardMode::Legacy);
    assert!(out.starts_with(&probe_request(&opts)));
    assert!(out.ends_with(&got.modes.enable));
}

fn no_read(_: Duration) -> io::Result<Option<Vec<u8>>> {
    panic!("must not read")
}

fn failing(_: Duration) -> io::Result<Option<Vec<u8>>> {
    Err(io::Error::other("closed"))
}

#[test]
fn negotiate_with_no_time_does_not_read() {
    let mut out = Vec::new();
    let opts = Options::default();
    let got = negotiate(&mut out, &mut no_read, &opts, Duration::ZERO).unwrap();
    assert_eq!(got.modes.keyboard, KeyboardMode::Legacy);
}

#[test]
fn negotiate_passes_read_errors_on() {
    let mut out = Vec::new();
    let opts = Options::default();
    let err = negotiate(&mut out, &mut failing, &opts, Duration::from_millis(200)).unwrap_err();
    assert_eq!(err.to_string(), "closed");
}
