//! Terminal abstraction: raw mode, input/output, event decoding.

#![forbid(unsafe_code)]

pub mod key;
pub mod key_test;

pub use key::{Key, KeyCode, Mods};

/// Event types emitted by terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A key was pressed.
    Key(Key),
    /// Text was pasted.
    Paste(String),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Worker thread woke up the event loop.
    Wake,
}
