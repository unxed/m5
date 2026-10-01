//! Win32-compatible keyboard event types, compatible with far2l and f4.
//!
//! This module defines event types that match the Win32 INPUT_RECORD format used by
//! far2l, f4, and other terminal applications. This ensures seamless integration and
//! correct handling of all keyboard input protocols (ansi, kitty, far2l APC, etc.).

use std::fmt;

/// Control key state flags (matches Win32 dwControlKeyState).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ControlKeyState(pub u32);

impl ControlKeyState {
    pub const RIGHT_ALT_PRESSED: u32 = 0x0001;
    pub const LEFT_ALT_PRESSED: u32 = 0x0002;
    pub const RIGHT_CTRL_PRESSED: u32 = 0x0004;
    pub const LEFT_CTRL_PRESSED: u32 = 0x0008;
    pub const SHIFT_PRESSED: u32 = 0x0010;
    pub const NUM_LOCK_ON: u32 = 0x0020;
    pub const SCROLL_LOCK_ON: u32 = 0x0040;
    pub const CAPS_LOCK_ON: u32 = 0x0080;
    pub const ENHANCED_KEY: u32 = 0x0100;

    pub fn new() -> Self {
        ControlKeyState(0)
    }

    pub fn with_left_alt(mut self) -> Self {
        self.0 |= Self::LEFT_ALT_PRESSED;
        self
    }

    pub fn with_right_alt(mut self) -> Self {
        self.0 |= Self::RIGHT_ALT_PRESSED;
        self
    }

    pub fn with_left_ctrl(mut self) -> Self {
        self.0 |= Self::LEFT_CTRL_PRESSED;
        self
    }

    pub fn with_right_ctrl(mut self) -> Self {
        self.0 |= Self::RIGHT_CTRL_PRESSED;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.0 |= Self::SHIFT_PRESSED;
        self
    }

    pub fn has_alt(&self) -> bool {
        (self.0 & (Self::LEFT_ALT_PRESSED | Self::RIGHT_ALT_PRESSED)) != 0
    }

    pub fn has_ctrl(&self) -> bool {
        (self.0 & (Self::LEFT_CTRL_PRESSED | Self::RIGHT_CTRL_PRESSED)) != 0
    }

    pub fn has_shift(&self) -> bool {
        (self.0 & Self::SHIFT_PRESSED) != 0
    }

    pub fn contains(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

impl Default for ControlKeyState {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ControlKeyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "None")
        } else {
            let mut parts = Vec::new();
            if self.contains(Self::RIGHT_ALT_PRESSED) {
                parts.push("RightAlt");
            }
            if self.contains(Self::LEFT_ALT_PRESSED) {
                parts.push("Alt");
            }
            if self.contains(Self::RIGHT_CTRL_PRESSED) {
                parts.push("RightCtrl");
            }
            if self.contains(Self::LEFT_CTRL_PRESSED) {
                parts.push("Ctrl");
            }
            if self.contains(Self::SHIFT_PRESSED) {
                parts.push("Shift");
            }
            if self.contains(Self::NUM_LOCK_ON) {
                parts.push("NumLock");
            }
            if self.contains(Self::SCROLL_LOCK_ON) {
                parts.push("ScrollLock");
            }
            if self.contains(Self::CAPS_LOCK_ON) {
                parts.push("CapsLock");
            }
            if self.contains(Self::ENHANCED_KEY) {
                parts.push("Enhanced");
            }
            write!(f, "{}", parts.join(","))
        }
    }
}

/// Event type constants.
///
/// `Key` and `Mouse` match Win32 INPUT_RECORD.EventType (KEY_EVENT, MOUSE_EVENT) and
/// `Focus` matches FOCUS_EVENT. `Paste`, `Far2l` and `Resize` follow the values used by
/// unxed/winkeys (the D-03 reference); they are extensions, not Win32 values
/// (Win32 WINDOW_BUFFER_SIZE_EVENT is 0x0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum EventType {
    Key = 0x0001,
    Mouse = 0x0002,
    Focus = 0x0010,
    Paste = 0x0020,
    Far2l = 0x0040,
    Resize = 0x0080,
}

/// Mouse button state flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonState(pub u32);

impl MouseButtonState {
    pub const FROM_LEFT_1ST_BUTTON_PRESSED: u32 = 0x0001;
    pub const RIGHTMOST_BUTTON_PRESSED: u32 = 0x0002;
    pub const FROM_LEFT_2ND_BUTTON_PRESSED: u32 = 0x0004;
    pub const FROM_LEFT_3RD_BUTTON_PRESSED: u32 = 0x0008;
    pub const FROM_LEFT_4TH_BUTTON_PRESSED: u32 = 0x0010;
}

/// Mouse event flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEventFlags(pub u32);

impl MouseEventFlags {
    pub const MOVED: u32 = 0x0001;
    pub const DOUBLE_CLICK: u32 = 0x0002;
    pub const WHEELED: u32 = 0x0004;
    pub const HWHEELED: u32 = 0x0008;
}

/// Generic input event (Key, Mouse, Focus, Paste, Far2l, Resize).
///
/// This structure matches the Win32 INPUT_RECORD format used by far2l and f4,
/// allowing seamless integration with existing keymap and input handling code.
#[derive(Debug, Clone)]
pub struct InputEvent {
    pub event_type: EventType,

    // Key Event Data
    pub virtual_key_code: u16,
    pub virtual_scan_code: u16,
    pub char_code: char,
    pub unshifted_char: char,
    pub key_down: bool,
    pub repeat_count: u16,

    // Mouse Event Data
    pub mouse_x: i16,
    pub mouse_y: i16,
    pub button_state: u32,
    pub mouse_event_flags: u32,
    pub wheel_direction: i32, // 1: forward/up, -1: backward/down

    // Focus Event Data
    pub set_focus: bool,

    // Paste Event Data
    pub paste_start: bool,

    // Far2l Extension Data
    pub far2l_command: String,
    pub far2l_data: Vec<u8>,

    // Shared
    pub control_key_state: ControlKeyState,

    // Input source (for debugging/logging)
    pub input_source: String,

    // Legacy indicator (from protocols without explicit KeyUp events)
    pub is_legacy: bool,
}

impl InputEvent {
    /// Blank event of the given type; all payload fields zeroed.
    fn blank(event_type: EventType) -> Self {
        InputEvent {
            event_type,
            virtual_key_code: 0,
            virtual_scan_code: 0,
            char_code: '\0',
            unshifted_char: '\0',
            key_down: false,
            repeat_count: 0,
            mouse_x: 0,
            mouse_y: 0,
            button_state: 0,
            mouse_event_flags: 0,
            wheel_direction: 0,
            set_focus: false,
            paste_start: false,
            far2l_command: String::new(),
            far2l_data: Vec::new(),
            control_key_state: ControlKeyState::new(),
            input_source: String::new(),
            is_legacy: false,
        }
    }

    /// Create a new key event (repeat count 1, no modifiers).
    pub fn key(vk: u16, scan: u16, ch: char, key_down: bool) -> Self {
        InputEvent {
            virtual_key_code: vk,
            virtual_scan_code: scan,
            char_code: ch,
            key_down,
            repeat_count: 1,
            ..Self::blank(EventType::Key)
        }
    }

    /// Create a new mouse event.
    pub fn mouse(x: i16, y: i16, buttons: u32, flags: u32) -> Self {
        InputEvent {
            mouse_x: x,
            mouse_y: y,
            button_state: buttons,
            mouse_event_flags: flags,
            ..Self::blank(EventType::Mouse)
        }
    }

    /// Create a new focus event.
    pub fn focus(set_focus: bool) -> Self {
        InputEvent {
            set_focus,
            ..Self::blank(EventType::Focus)
        }
    }

    /// Create a new paste event.
    pub fn paste(paste_start: bool) -> Self {
        InputEvent {
            paste_start,
            ..Self::blank(EventType::Paste)
        }
    }

    /// Create a resize event.
    pub fn resize() -> Self {
        Self::blank(EventType::Resize)
    }

    /// Set the unshifted character (kitty-style "base" key).
    pub fn with_unshifted_char(mut self, ch: char) -> Self {
        self.unshifted_char = ch;
        self
    }

    pub fn with_control_state(mut self, state: ControlKeyState) -> Self {
        self.control_key_state = state;
        self
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.input_source = source;
        self
    }
}

impl fmt::Display for InputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.event_type {
            EventType::Key => {
                let state = if self.key_down { "DOWN" } else { "UP" };
                write!(
                    f,
                    "Key{{VK:0x{:04X} Scan:0x{:04X} Char:'{}' {} Mods:{} Src:{}}}",
                    self.virtual_key_code,
                    self.virtual_scan_code,
                    if self.char_code > '\0' && self.char_code as u32 >= 32 {
                        self.char_code.to_string()
                    } else {
                        format!("\\x{:02X}", self.char_code as u32)
                    },
                    state,
                    self.control_key_state,
                    self.input_source
                )
            }
            EventType::Mouse => {
                write!(
                    f,
                    "Mouse{{Pos:({},{}) Buttons:0x{:04X} Flags:0x{:04X} Mods:{}}}",
                    self.mouse_x, self.mouse_y, self.button_state, self.mouse_event_flags, self.control_key_state
                )
            }
            EventType::Focus => {
                write!(
                    f,
                    "Focus{{{}}}",
                    if self.set_focus { "IN" } else { "OUT" }
                )
            }
            EventType::Paste => {
                write!(
                    f,
                    "Paste{{{}}}",
                    if self.paste_start { "START" } else { "END" }
                )
            }
            EventType::Far2l => {
                write!(
                    f,
                    "Far2l{{{} len:{}}}",
                    self.far2l_command,
                    self.far2l_data.len()
                )
            }
            EventType::Resize => {
                write!(f, "Resize{{}}")
            }
        }
    }
}

/// Win32 virtual key codes (subset; values per Microsoft docs and unxed/winkeys vkeys.go).
pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12; // Alt
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21; // Page Up
    pub const NEXT: u16 = 0x22; // Page Down
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const A: u16 = 0x41;
    pub const Z: u16 = 0x5A;
    pub const F1: u16 = 0x70;
    pub const F4: u16 = 0x73;
    pub const F12: u16 = 0x7B;
    pub const F24: u16 = 0x87;
}

/// Virtual key code for a printable ASCII character: letters map to the uppercase
/// letter code (0x41..=0x5A), digits to themselves, space to VK_SPACE; otherwise 0
/// (unknown, to be resolved by a real decoder).
pub fn vk_from_ascii(c: char) -> u16 {
    match c {
        'a'..='z' => c.to_ascii_uppercase() as u16,
        'A'..='Z' | '0'..='9' => c as u16,
        ' ' => vk::SPACE,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_key_state_flag_values_match_win32() {
        assert_eq!(ControlKeyState::RIGHT_ALT_PRESSED, 0x0001);
        assert_eq!(ControlKeyState::LEFT_ALT_PRESSED, 0x0002);
        assert_eq!(ControlKeyState::RIGHT_CTRL_PRESSED, 0x0004);
        assert_eq!(ControlKeyState::LEFT_CTRL_PRESSED, 0x0008);
        assert_eq!(ControlKeyState::SHIFT_PRESSED, 0x0010);
        assert_eq!(ControlKeyState::NUM_LOCK_ON, 0x0020);
        assert_eq!(ControlKeyState::SCROLL_LOCK_ON, 0x0040);
        assert_eq!(ControlKeyState::CAPS_LOCK_ON, 0x0080);
        assert_eq!(ControlKeyState::ENHANCED_KEY, 0x0100);
    }

    #[test]
    fn mouse_constants_match_win32() {
        assert_eq!(MouseButtonState::FROM_LEFT_1ST_BUTTON_PRESSED, 0x0001);
        assert_eq!(MouseButtonState::RIGHTMOST_BUTTON_PRESSED, 0x0002);
        assert_eq!(MouseButtonState::FROM_LEFT_2ND_BUTTON_PRESSED, 0x0004);
        assert_eq!(MouseButtonState::FROM_LEFT_3RD_BUTTON_PRESSED, 0x0008);
        assert_eq!(MouseButtonState::FROM_LEFT_4TH_BUTTON_PRESSED, 0x0010);
        assert_eq!(MouseEventFlags::MOVED, 0x0001);
        assert_eq!(MouseEventFlags::DOUBLE_CLICK, 0x0002);
        assert_eq!(MouseEventFlags::WHEELED, 0x0004);
        assert_eq!(MouseEventFlags::HWHEELED, 0x0008);
    }

    #[test]
    fn event_type_values() {
        assert_eq!(EventType::Key as u16, 0x0001);
        assert_eq!(EventType::Mouse as u16, 0x0002);
        assert_eq!(EventType::Focus as u16, 0x0010);
        assert_eq!(EventType::Paste as u16, 0x0020);
        assert_eq!(EventType::Far2l as u16, 0x0040);
        assert_eq!(EventType::Resize as u16, 0x0080);
    }

    #[test]
    fn vk_constants_match_win32() {
        assert_eq!(vk::BACK, 0x08);
        assert_eq!(vk::TAB, 0x09);
        assert_eq!(vk::RETURN, 0x0D);
        assert_eq!(vk::SHIFT, 0x10);
        assert_eq!(vk::CONTROL, 0x11);
        assert_eq!(vk::MENU, 0x12);
        assert_eq!(vk::ESCAPE, 0x1B);
        assert_eq!(vk::SPACE, 0x20);
        assert_eq!(vk::PRIOR, 0x21);
        assert_eq!(vk::NEXT, 0x22);
        assert_eq!(vk::END, 0x23);
        assert_eq!(vk::HOME, 0x24);
        assert_eq!(vk::LEFT, 0x25);
        assert_eq!(vk::UP, 0x26);
        assert_eq!(vk::RIGHT, 0x27);
        assert_eq!(vk::DOWN, 0x28);
        assert_eq!(vk::INSERT, 0x2D);
        assert_eq!(vk::DELETE, 0x2E);
        assert_eq!(vk::A, 'A' as u16);
        assert_eq!(vk::Z, 'Z' as u16);
        assert_eq!(vk::F1, 0x70);
        assert_eq!(vk::F4, 0x73);
        assert_eq!(vk::F12, 0x7B);
        assert_eq!(vk::F24, vk::F1 + 23);
    }

    #[test]
    fn vk_from_ascii_maps_letters_to_uppercase_codes() {
        assert_eq!(vk_from_ascii('a'), vk::A);
        assert_eq!(vk_from_ascii('Z'), vk::Z);
        assert_eq!(vk_from_ascii('5'), 0x35);
        assert_eq!(vk_from_ascii(' '), vk::SPACE);
        assert_eq!(vk_from_ascii('!'), 0);
    }

    #[test]
    fn control_key_state_modifier_queries() {
        let cks = ControlKeyState::new().with_left_ctrl().with_shift();
        assert!(cks.has_ctrl());
        assert!(cks.has_shift());
        assert!(!cks.has_alt());
        assert_eq!(
            cks.0,
            ControlKeyState::LEFT_CTRL_PRESSED | ControlKeyState::SHIFT_PRESSED
        );
    }

    #[test]
    fn right_and_left_modifiers_are_distinct_bits() {
        let left = ControlKeyState::new().with_left_alt();
        let right = ControlKeyState::new().with_right_alt();
        assert!(left.has_alt() && right.has_alt());
        assert!(left.contains(ControlKeyState::LEFT_ALT_PRESSED));
        assert!(!left.contains(ControlKeyState::RIGHT_ALT_PRESSED));
        assert!(right.contains(ControlKeyState::RIGHT_ALT_PRESSED));
        assert!(!right.contains(ControlKeyState::LEFT_ALT_PRESSED));
        assert_ne!(left, right);
    }

    #[test]
    fn control_key_state_display() {
        assert_eq!(ControlKeyState::new().to_string(), "None");
        let cks = ControlKeyState::new().with_left_ctrl().with_shift();
        assert_eq!(cks.to_string(), "Ctrl,Shift");
        let cks = ControlKeyState::new().with_right_ctrl();
        assert_eq!(cks.to_string(), "RightCtrl");
        let cks = ControlKeyState(ControlKeyState::ENHANCED_KEY | ControlKeyState::NUM_LOCK_ON);
        assert_eq!(cks.to_string(), "NumLock,Enhanced");
    }

    #[test]
    fn key_constructor_defaults() {
        let ev = InputEvent::key(vk::A, 0x1E, 'a', true);
        assert_eq!(ev.event_type, EventType::Key);
        assert_eq!(ev.virtual_key_code, vk::A);
        assert_eq!(ev.virtual_scan_code, 0x1E);
        assert_eq!(ev.char_code, 'a');
        assert_eq!(ev.unshifted_char, '\0');
        assert!(ev.key_down);
        assert_eq!(ev.repeat_count, 1);
        assert_eq!(ev.control_key_state, ControlKeyState::default());
        assert!(!ev.is_legacy);
    }

    #[test]
    fn key_builder_methods() {
        let ev = InputEvent::key(vk::A, 0x1E, 'A', true)
            .with_control_state(ControlKeyState::new().with_shift())
            .with_unshifted_char('a')
            .with_source("test".to_string());
        assert_eq!(ev.char_code, 'A');
        assert_eq!(ev.unshifted_char, 'a');
        assert!(ev.control_key_state.has_shift());
        assert_eq!(ev.input_source, "test");
    }

    #[test]
    fn non_key_constructors_set_only_their_payload() {
        let m = InputEvent::mouse(10, 20, MouseButtonState::RIGHTMOST_BUTTON_PRESSED, 0);
        assert_eq!(m.event_type, EventType::Mouse);
        assert_eq!((m.mouse_x, m.mouse_y), (10, 20));
        assert_eq!(m.button_state, MouseButtonState::RIGHTMOST_BUTTON_PRESSED);
        assert_eq!(m.virtual_key_code, 0);
        assert_eq!(m.repeat_count, 0);

        assert!(InputEvent::focus(true).set_focus);
        assert!(!InputEvent::focus(false).set_focus);
        assert!(InputEvent::paste(true).paste_start);
        assert!(!InputEvent::paste(false).paste_start);
        assert_eq!(InputEvent::resize().event_type, EventType::Resize);
    }

    #[test]
    fn display_key_event() {
        let ev = InputEvent::key(vk::A, 0x1E, 'A', true)
            .with_control_state(ControlKeyState::new().with_left_ctrl().with_shift())
            .with_source("unix_raw".to_string());
        assert_eq!(
            ev.to_string(),
            "Key{VK:0x0041 Scan:0x001E Char:'A' DOWN Mods:Ctrl,Shift Src:unix_raw}"
        );
    }

    #[test]
    fn display_key_event_control_char_and_up() {
        let ev = InputEvent::key(vk::ESCAPE, 0x01, '\x1B', false);
        assert_eq!(
            ev.to_string(),
            "Key{VK:0x001B Scan:0x0001 Char:'\\x1B' UP Mods:None Src:}"
        );
    }

    #[test]
    fn display_other_events() {
        assert_eq!(
            InputEvent::mouse(15, 25, 1, 1).to_string(),
            "Mouse{Pos:(15,25) Buttons:0x0001 Flags:0x0001 Mods:None}"
        );
        assert_eq!(InputEvent::focus(true).to_string(), "Focus{IN}");
        assert_eq!(InputEvent::focus(false).to_string(), "Focus{OUT}");
        assert_eq!(InputEvent::paste(true).to_string(), "Paste{START}");
        assert_eq!(InputEvent::paste(false).to_string(), "Paste{END}");
        assert_eq!(InputEvent::resize().to_string(), "Resize{}");
    }

    #[test]
    fn display_far2l_event() {
        let mut ev = InputEvent::resize();
        ev.event_type = EventType::Far2l;
        ev.far2l_command = "F2L_K".to_string();
        ev.far2l_data = vec![1, 2, 3, 4];
        assert_eq!(ev.to_string(), "Far2l{F2L_K len:4}");
    }
}
