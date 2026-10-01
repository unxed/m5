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

/// Event type constants (matches Win32 INPUT_RECORD.EventType).
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
    /// Create a new key event.
    pub fn key(vk: u16, scan: u16, ch: char, key_down: bool) -> Self {
        InputEvent {
            event_type: EventType::Key,
            virtual_key_code: vk,
            virtual_scan_code: scan,
            char_code: ch,
            unshifted_char: '\0',
            key_down,
            repeat_count: 1,
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

    /// Create a new mouse event.
    pub fn mouse(x: i16, y: i16, buttons: u32, flags: u32) -> Self {
        InputEvent {
            event_type: EventType::Mouse,
            virtual_key_code: 0,
            virtual_scan_code: 0,
            char_code: '\0',
            unshifted_char: '\0',
            key_down: false,
            repeat_count: 0,
            mouse_x: x,
            mouse_y: y,
            button_state: buttons,
            mouse_event_flags: flags,
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

    /// Create a new focus event.
    pub fn focus(set_focus: bool) -> Self {
        InputEvent {
            event_type: EventType::Focus,
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
            set_focus,
            paste_start: false,
            far2l_command: String::new(),
            far2l_data: Vec::new(),
            control_key_state: ControlKeyState::new(),
            input_source: String::new(),
            is_legacy: false,
        }
    }

    /// Create a new paste event.
    pub fn paste(paste_start: bool) -> Self {
        InputEvent {
            event_type: EventType::Paste,
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
            paste_start,
            far2l_command: String::new(),
            far2l_data: Vec::new(),
            control_key_state: ControlKeyState::new(),
            input_source: String::new(),
            is_legacy: false,
        }
    }

    /// Create a resize event.
    pub fn resize() -> Self {
        InputEvent {
            event_type: EventType::Resize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_key_state_flags() {
        let cks = ControlKeyState::new()
            .with_left_ctrl()
            .with_shift();
        assert!(cks.has_ctrl());
        assert!(cks.has_shift());
        assert!(!cks.has_alt());
    }

    #[test]
    fn test_key_event_creation() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true)
            .with_control_state(ControlKeyState::new().with_left_ctrl());
        assert_eq!(event.event_type, EventType::Key);
        assert_eq!(event.virtual_key_code, 0x41);
        assert!(event.key_down);
        assert!(event.control_key_state.has_ctrl());
    }

    #[test]
    fn test_display_format() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true);
        let display = format!("{}", event);
        assert!(display.contains("Key{"));
        assert!(display.contains("VK:0x0041"));
    }
}
