//! Windows virtual-key constants and layer-key names.

use crate::keymap::{EventFlags, KeyCode};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LCONTROL, VK_LEFT, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7,
    VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_RCONTROL, VK_RETURN, VK_RIGHT,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SPACE, VK_TAB, VK_UP,
};

pub(crate) const KEY_H: KeyCode = b'H' as KeyCode;
pub(crate) const KEY_J: KeyCode = b'J' as KeyCode;
pub(crate) const KEY_K: KeyCode = b'K' as KeyCode;
pub(crate) const KEY_L: KeyCode = b'L' as KeyCode;
pub(crate) const KEY_SEMICOLON: KeyCode = VK_OEM_1;

pub(crate) const KEY_LEFT_ARROW: KeyCode = VK_LEFT;
pub(crate) const KEY_UP_ARROW: KeyCode = VK_UP;
pub(crate) const KEY_RIGHT_ARROW: KeyCode = VK_RIGHT;
pub(crate) const KEY_DOWN_ARROW: KeyCode = VK_DOWN;

/// Windows uses synthetic `Ctrl` key events for shortcuts rather than
/// rewriting a flag on the intercepted event. This flag is still used by pure
/// tests to verify the platform shortcut modifier helper.
#[allow(dead_code)]
pub(crate) const SHORTCUT_MODIFIER_FLAG_MASK: EventFlags = 1 << 18;
pub(crate) const SHORTCUT_MODIFIER_NAME: &str = "Control";
pub(crate) const PLATFORM_KEY_CODE_NAME: &str = "Windows virtual key code";
pub(crate) const SHORTCUT_MODIFIER_KEY: KeyCode = VK_CONTROL;

/// Friendly names accepted for the layer key, paired with Windows virtual-key
/// codes. The punctuation names assume the common US-position OEM virtual keys;
/// Windows maps those virtual keys through the active keyboard layout.
pub(crate) const LAYER_KEY_NAMES: &[(&str, KeyCode)] = &[
    ("semicolon", VK_OEM_1),
    ("quote", VK_OEM_7),
    ("apostrophe", VK_OEM_7),
    ("grave", VK_OEM_3),
    ("backtick", VK_OEM_3),
    ("tab", VK_TAB),
    ("return", VK_RETURN),
    ("enter", VK_RETURN),
    ("space", VK_SPACE),
    ("escape", VK_ESCAPE),
    ("delete", VK_DELETE),
    ("backspace", VK_BACK),
    ("backslash", VK_OEM_5),
    ("left_bracket", VK_OEM_4),
    ("right_bracket", VK_OEM_6),
    ("comma", VK_OEM_COMMA),
    ("period", VK_OEM_PERIOD),
    ("slash", VK_OEM_2),
    ("minus", VK_OEM_MINUS),
    ("equal", VK_OEM_PLUS),
    // Cross-platform compatibility aliases.
    ("left_command", VK_LWIN),
    ("right_command", VK_RWIN),
    ("left_option", VK_LMENU),
    ("right_option", VK_RMENU),
    // Windows-native aliases.
    ("left_windows", VK_LWIN),
    ("right_windows", VK_RWIN),
    ("left_win", VK_LWIN),
    ("right_win", VK_RWIN),
    ("left_alt", VK_LMENU),
    ("right_alt", VK_RMENU),
    ("left_control", VK_LCONTROL),
    ("right_control", VK_RCONTROL),
    ("left_ctrl", VK_LCONTROL),
    ("right_ctrl", VK_RCONTROL),
    ("left_shift", VK_LSHIFT),
    ("right_shift", VK_RSHIFT),
];

pub(crate) const MODIFIER_KEY_CODES: &[KeyCode] = &[
    VK_LWIN,
    VK_RWIN,
    VK_LSHIFT,
    VK_RSHIFT,
    VK_LCONTROL,
    VK_RCONTROL,
    VK_LMENU,
    VK_RMENU,
];

pub(crate) const MODIFIER_DEVICE_FLAGS: &[(KeyCode, EventFlags)] = &[];
pub(crate) const MODIFIER_CLEAR_MASKS: &[(KeyCode, EventFlags)] = &[];
pub(crate) const UNSUPPORTED_MODIFIER_CODES: &[KeyCode] = &[VK_CAPITAL];
