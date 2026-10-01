//! Terminal abstraction: raw mode, input/output, event decoding.

#![forbid(unsafe_code)]

pub mod key;

pub use key::{Key, KeyCode, Mods};
