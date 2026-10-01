//! Terminal abstraction: raw mode, input/output, event decoding.
//!
//! Uses Win32-compatible InputEvent format (like far2l and f4) for cross-platform
//! keyboard event handling that works seamlessly with all input protocols.

#![forbid(unsafe_code)]

pub mod decode;
pub mod key;
pub mod key_test;

pub use decode::Decoder;

pub use key::{
    ControlKeyState, EventType, InputEvent, MouseButtonState, MouseEventFlags, vk, vk_from_ascii,
};
