//! Key and modifier types for terminal input.

/// Modifier keys as a bitmask: SHIFT=1, ALT=2, CTRL=4, SUPER=8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mods(pub u8);

impl Mods {
    pub const SHIFT: u8 = 1;
    pub const ALT: u8 = 2;
    pub const CTRL: u8 = 4;
    pub const SUPER: u8 = 8;

    pub fn new() -> Self {
        Mods(0)
    }

    pub fn with_shift(mut self) -> Self {
        self.0 |= Self::SHIFT;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.0 |= Self::ALT;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.0 |= Self::CTRL;
        self
    }

    pub fn with_super(mut self) -> Self {
        self.0 |= Self::SUPER;
        self
    }

    pub fn is_shift(&self) -> bool {
        self.0 & Self::SHIFT != 0
    }

    pub fn is_alt(&self) -> bool {
        self.0 & Self::ALT != 0
    }

    pub fn is_ctrl(&self) -> bool {
        self.0 & Self::CTRL != 0
    }

    pub fn is_super(&self) -> bool {
        self.0 & Self::SUPER != 0
    }
}

impl Default for Mods {
    fn default() -> Self {
        Self::new()
    }
}

/// Special key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Printable character; for Ctrl/Alt variants, the base letter is stored in lowercase.
    Char(char),
    /// Function keys F1..=F24.
    F(u8),
    /// Arrow and navigation keys.
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Numeric keypad variants (after MVP).
    Insert,
    Delete,
    /// Standard control keys.
    Enter,
    Tab,
    Backspace,
    Esc,
}

/// A key press with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub mods: Mods,
}

impl Key {
    pub fn new(code: KeyCode) -> Self {
        Key {
            code,
            mods: Mods::new(),
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.mods = self.mods.with_shift();
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.mods = self.mods.with_alt();
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.mods = self.mods.with_ctrl();
        self
    }

    pub fn with_super(mut self) -> Self {
        self.mods = self.mods.with_super();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mods_bitflags() {
        let mods = Mods::new().with_ctrl().with_alt();
        assert!(mods.is_ctrl());
        assert!(mods.is_alt());
        assert!(!mods.is_shift());
        assert!(!mods.is_super());
    }

    #[test]
    fn test_key_creation() {
        let key = Key::new(KeyCode::Char('a')).with_ctrl().with_alt();
        assert!(key.mods.is_ctrl());
        assert!(key.mods.is_alt());
        assert!(!key.mods.is_shift());
    }
}
