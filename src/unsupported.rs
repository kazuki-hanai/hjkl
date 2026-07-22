//! Fallback backend for operating systems without a keyboard-hook implementation.
//!
//! The common CLI and keymap still compile here, which keeps tests useful on
//! development hosts that are neither macOS nor Windows. Runtime commands
//! return a clear unsupported-platform error.

use crate::error::{Error, Result};
use crate::keymap::{EventFlags, KeyCode};

pub(crate) mod keys {
    #![allow(dead_code)]

    use super::{EventFlags, KeyCode};

    pub(crate) const KEY_H: KeyCode = b'H' as KeyCode;
    pub(crate) const KEY_J: KeyCode = b'J' as KeyCode;
    pub(crate) const KEY_K: KeyCode = b'K' as KeyCode;
    pub(crate) const KEY_L: KeyCode = b'L' as KeyCode;
    pub(crate) const KEY_SEMICOLON: KeyCode = 0xBA;

    pub(crate) const KEY_LEFT_ARROW: KeyCode = 0x25;
    pub(crate) const KEY_UP_ARROW: KeyCode = 0x26;
    pub(crate) const KEY_RIGHT_ARROW: KeyCode = 0x27;
    pub(crate) const KEY_DOWN_ARROW: KeyCode = 0x28;

    pub(crate) const SHORTCUT_MODIFIER_FLAG_MASK: EventFlags = 1 << 18;
    pub(crate) const SHORTCUT_MODIFIER_NAME: &str = "Control";
    pub(crate) const PLATFORM_KEY_CODE_NAME: &str = "virtual key code";

    pub(crate) const LAYER_KEY_NAMES: &[(&str, KeyCode)] = &[
        ("semicolon", 0xBA),
        ("quote", 0xDE),
        ("apostrophe", 0xDE),
        ("grave", 0xC0),
        ("backtick", 0xC0),
        ("tab", 0x09),
        ("return", 0x0D),
        ("enter", 0x0D),
        ("space", 0x20),
        ("escape", 0x1B),
        ("delete", 0x2E),
        ("backspace", 0x08),
        ("backslash", 0xDC),
        ("left_bracket", 0xDB),
        ("right_bracket", 0xDD),
        ("comma", 0xBC),
        ("period", 0xBE),
        ("slash", 0xBF),
        ("minus", 0xBD),
        ("equal", 0xBB),
        ("left_command", 0x5B),
        ("right_command", 0x5C),
        ("left_windows", 0x5B),
        ("right_windows", 0x5C),
        ("left_win", 0x5B),
        ("right_win", 0x5C),
        ("left_option", 0xA4),
        ("right_option", 0xA5),
        ("left_alt", 0xA4),
        ("right_alt", 0xA5),
        ("left_control", 0xA2),
        ("right_control", 0xA3),
        ("left_ctrl", 0xA2),
        ("right_ctrl", 0xA3),
        ("left_shift", 0xA0),
        ("right_shift", 0xA1),
    ];

    pub(crate) const MODIFIER_KEY_CODES: &[KeyCode] =
        &[0x5B, 0x5C, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5];
    pub(crate) const MODIFIER_DEVICE_FLAGS: &[(KeyCode, EventFlags)] = &[];
    pub(crate) const MODIFIER_CLEAR_MASKS: &[(KeyCode, EventFlags)] = &[];
    pub(crate) const UNSUPPORTED_MODIFIER_CODES: &[KeyCode] = &[0x14];
}

pub(crate) fn run_event_loop(_service_mode: bool) -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn start(_layer_key: Option<KeyCode>) -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn stop() -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn restart(_layer_key: Option<KeyCode>) -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn enable(_layer_key: Option<KeyCode>) -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn disable() -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn status() -> Result<()> {
    Err(unsupported_error())
}

pub(crate) fn request_permissions() -> Result<()> {
    Err(unsupported_error())
}

fn unsupported_error() -> Error {
    Error::from("hjkl currently supports macOS and Windows.")
}
