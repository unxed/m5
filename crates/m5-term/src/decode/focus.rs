//! Focus reports (xterm mode 1004): `CSI I` when the terminal window gains the focus and
//! `CSI O` when it loses it.
//!
//! Known limit: some terminals (VTE, the FreeBSD console) write the modified F1..F4 as
//! `ESC [ O digit P` instead of `ESC O digit P`; that form starts like a focus-out report and
//! is not recognized as a key here.

use super::csi::Csi;
use crate::key::InputEvent;

/// Turns focus reports on.
pub const ENABLE: &[u8] = b"\x1b[?1004h";
/// Turns focus reports off.
pub const DISABLE: &[u8] = b"\x1b[?1004l";

/// The focus event of `CSI I` or `CSI O` (no parameters, no private marker).
pub(crate) fn event(csi: &Csi) -> Option<InputEvent> {
    let bare = csi.private.is_none()
        && !csi.has_intermediate
        && csi.params.len() == 1
        && csi.param(0).is_none();
    if !bare {
        return None;
    }
    let gained = match csi.final_byte {
        b'I' => true,
        b'O' => false,
        _ => return None,
    };
    let mut ev = InputEvent::focus(gained);
    ev.input_source = "focus".to_string();
    Some(ev)
}
