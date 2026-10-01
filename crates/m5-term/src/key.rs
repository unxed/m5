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

    // ============= ControlKeyState Tests =============

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
    fn test_control_key_state_all_modifiers() {
        let cks = ControlKeyState::new()
            .with_left_ctrl()
            .with_right_ctrl()
            .with_left_alt()
            .with_right_alt()
            .with_shift();

        assert!(cks.has_ctrl());
        assert!(cks.has_alt());
        assert!(cks.has_shift());
        assert!(cks.contains(ControlKeyState::LEFT_CTRL_PRESSED));
        assert!(cks.contains(ControlKeyState::RIGHT_CTRL_PRESSED));
        assert!(cks.contains(ControlKeyState::LEFT_ALT_PRESSED));
        assert!(cks.contains(ControlKeyState::RIGHT_ALT_PRESSED));
        assert!(cks.contains(ControlKeyState::SHIFT_PRESSED));
    }

    #[test]
    fn test_control_key_state_display() {
        let cks = ControlKeyState::new()
            .with_left_ctrl()
            .with_shift();
        let display = format!("{}", cks);
        assert!(display.contains("Ctrl"));
        assert!(display.contains("Shift"));
    }

    #[test]
    fn test_control_key_state_empty() {
        let cks = ControlKeyState::new();
        let display = format!("{}", cks);
        assert_eq!(display, "None");
    }

    // ============= Key Event Tests =============

    #[test]
    fn test_key_event_creation() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true)
            .with_control_state(ControlKeyState::new().with_left_ctrl());
        assert_eq!(event.event_type, EventType::Key);
        assert_eq!(event.virtual_key_code, 0x41);
        assert_eq!(event.virtual_scan_code, 0x1E);
        assert_eq!(event.char_code, 'A');
        assert!(event.key_down);
        assert!(event.control_key_state.has_ctrl());
    }

    #[test]
    fn test_key_event_key_up() {
        let event = InputEvent::key(0x41, 0x1E, 'A', false);
        assert!(!event.key_down);
    }

    #[test]
    fn test_key_event_repeat_count() {
        let mut event = InputEvent::key(0x41, 0x1E, 'A', true);
        event.repeat_count = 5;
        assert_eq!(event.repeat_count, 5);
    }

    #[test]
    fn test_key_event_special_char_escape() {
        let event = InputEvent::key(0x1B, 0x01, '\x1B', true); // ESC key
        assert_eq!(event.char_code, '\x1B');
        assert_eq!(event.virtual_key_code, 0x1B);
    }

    #[test]
    fn test_key_event_ctrl_a() {
        let event = InputEvent::key(0x41, 0x1E, 'a', true)
            .with_control_state(ControlKeyState::new().with_left_ctrl());
        assert!(event.control_key_state.has_ctrl());
        assert_eq!(event.char_code, 'a');
        let display = format!("{}", event);
        assert!(display.contains("Ctrl"));
    }

    #[test]
    fn test_key_event_shift_up() {
        let event = InputEvent::key(0x26, 0x48, '^', true) // Up arrow with Shift
            .with_control_state(ControlKeyState::new().with_shift());
        assert!(event.control_key_state.has_shift());
        assert_eq!(event.virtual_key_code, 0x26);
    }

    #[test]
    fn test_key_event_alt_f4() {
        let event = InputEvent::key(0x73, 0x3E, '\0', true) // F4 key
            .with_control_state(ControlKeyState::new().with_left_alt());
        assert!(event.control_key_state.has_alt());
        assert_eq!(event.virtual_key_code, 0x73);
    }

    #[test]
    fn test_key_event_enhanced_key_flag() {
        let cks = ControlKeyState::new()
            .with_shift();
        let cks_with_enhanced = ControlKeyState(cks.0 | ControlKeyState::ENHANCED_KEY);
        let event = InputEvent::key(0x26, 0x48, '\0', true) // Up arrow
            .with_control_state(cks_with_enhanced);
        assert!(event.control_key_state.contains(ControlKeyState::ENHANCED_KEY));
    }

    // ============= Mouse Event Tests =============

    #[test]
    fn test_mouse_event_creation() {
        let event = InputEvent::mouse(10, 20, MouseButtonState::FROM_LEFT_1ST_BUTTON_PRESSED, MouseEventFlags::MOVED);
        assert_eq!(event.event_type, EventType::Mouse);
        assert_eq!(event.mouse_x, 10);
        assert_eq!(event.mouse_y, 20);
        assert_eq!(event.button_state, MouseButtonState::FROM_LEFT_1ST_BUTTON_PRESSED);
    }

    // ============= Other Event Types =============

    #[test]
    fn test_focus_event_in() {
        let event = InputEvent::focus(true);
        assert_eq!(event.event_type, EventType::Focus);
        assert!(event.set_focus);
    }

    #[test]
    fn test_focus_event_out() {
        let event = InputEvent::focus(false);
        assert_eq!(event.event_type, EventType::Focus);
        assert!(!event.set_focus);
    }

    #[test]
    fn test_paste_event_start() {
        let event = InputEvent::paste(true);
        assert_eq!(event.event_type, EventType::Paste);
        assert!(event.paste_start);
    }

    #[test]
    fn test_paste_event_end() {
        let event = InputEvent::paste(false);
        assert_eq!(event.event_type, EventType::Paste);
        assert!(!event.paste_start);
    }

    #[test]
    fn test_resize_event() {
        let event = InputEvent::resize();
        assert_eq!(event.event_type, EventType::Resize);
    }

    // ============= Display Format Tests =============

    #[test]
    fn test_display_format_key_event() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true);
        let display = format!("{}", event);
        assert!(display.contains("Key{"));
        assert!(display.contains("VK:0x0041"));
        assert!(display.contains("DOWN"));
    }

    #[test]
    fn test_display_format_key_with_modifiers() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true)
            .with_control_state(ControlKeyState::new().with_left_ctrl().with_shift());
        let display = format!("{}", event);
        assert!(display.contains("Ctrl"));
        assert!(display.contains("Shift"));
    }

    #[test]
    fn test_display_format_mouse_event() {
        let event = InputEvent::mouse(15, 25, 0x0001, 0x0001);
        let display = format!("{}", event);
        assert!(display.contains("Mouse{"));
        assert!(display.contains("15"));
        assert!(display.contains("25"));
    }

    #[test]
    fn test_display_format_focus_event() {
        let event = InputEvent::focus(true);
        let display = format!("{}", event);
        assert!(display.contains("Focus{"));
        assert!(display.contains("IN"));
    }

    #[test]
    fn test_display_format_paste_event() {
        let event = InputEvent::paste(true);
        let display = format!("{}", event);
        assert!(display.contains("Paste{"));
        assert!(display.contains("START"));
    }

    #[test]
    fn test_display_format_resize_event() {
        let event = InputEvent::resize();
        let display = format!("{}", event);
        assert!(display.contains("Resize"));
    }

    // ============= Win32 Compatibility Tests =============

    #[test]
    fn test_win32_virtual_key_codes() {
        // Test common Win32 VK constants
        let vk_a = 0x41;      // VK_A
        let vk_enter = 0x0D;  // VK_RETURN
        let vk_shift = 0x10;  // VK_SHIFT
        let vk_ctrl = 0x11;   // VK_CONTROL
        let vk_alt = 0x12;    // VK_MENU
        let vk_escape = 0x1B; // VK_ESCAPE
        let vk_up = 0x26;     // VK_UP
        let vk_down = 0x28;   // VK_DOWN
        let vk_left = 0x25;   // VK_LEFT
        let vk_right = 0x27;  // VK_RIGHT
        let vk_f1 = 0x70;     // VK_F1
        let vk_f4 = 0x73;     // VK_F4

        assert_eq!(vk_a, 0x41);
        assert_eq!(vk_enter, 0x0D);
        assert_eq!(vk_escape, 0x1B);
        assert_eq!(vk_up, 0x26);

        // Create events with these codes
        let _ev_a = InputEvent::key(vk_a, 0x1E, 'a', true);
        let _ev_enter = InputEvent::key(vk_enter, 0x1C, '\n', true);
        let _ev_esc = InputEvent::key(vk_escape, 0x01, '\x1B', true);
        let _ev_f4 = InputEvent::key(vk_f4, 0x3E, '\0', true);
    }

    #[test]
    fn test_input_source_tracking() {
        let event = InputEvent::key(0x41, 0x1E, 'A', true)
            .with_source("unix_raw".to_string());
        assert_eq!(event.input_source, "unix_raw");
        let display = format!("{}", event);
        assert!(display.contains("unix_raw"));
    }

    #[test]
    fn test_legacy_flag() {
        let mut event = InputEvent::key(0x41, 0x1E, 'A', true);
        assert!(!event.is_legacy);
        event.is_legacy = true;
        assert!(event.is_legacy);
    }

    #[test]
    fn test_far2l_event() {
        let mut event = InputEvent::key(0x41, 0x1E, 'A', true);
        event.event_type = EventType::Far2l;
        event.far2l_command = "F2L_K".to_string();
        event.far2l_data = vec![0x41, 0x00, 0x00, 0x1E];

        assert_eq!(event.event_type, EventType::Far2l);
        assert_eq!(event.far2l_command, "F2L_K");
        assert_eq!(event.far2l_data.len(), 4);
    }

    // ============= Modifier Combination Tests =============

    #[test]
    fn test_ctrl_shift_combination() {
        let cks = ControlKeyState::new()
            .with_left_ctrl()
            .with_shift();
        assert!(cks.has_ctrl());
        assert!(cks.has_shift());
        assert!(!cks.has_alt());
    }

    #[test]
    fn test_ctrl_alt_shift_combination() {
        let cks = ControlKeyState::new()
            .with_left_ctrl()
            .with_left_alt()
            .with_shift();
        assert!(cks.has_ctrl());
        assert!(cks.has_alt());
        assert!(cks.has_shift());
    }

    #[test]
    fn test_right_vs_left_modifiers() {
        let left = ControlKeyState::new().with_left_ctrl();
        let right = ControlKeyState::new().with_right_ctrl();
        let both = ControlKeyState::new().with_left_ctrl().with_right_ctrl();

        assert!(left.has_ctrl());
        assert!(right.has_ctrl());
        assert!(both.has_ctrl());

        assert!(left.contains(ControlKeyState::LEFT_CTRL_PRESSED));
        assert!(right.contains(ControlKeyState::RIGHT_CTRL_PRESSED));
        assert!(both.contains(ControlKeyState::LEFT_CTRL_PRESSED));
        assert!(both.contains(ControlKeyState::RIGHT_CTRL_PRESSED));
    }
}
