//! macOS virtual-key constants and layer-key names.

use crate::keymap::{EventFlags, KeyCode};

pub(crate) const KEY_H: KeyCode = 4;
pub(crate) const KEY_L: KeyCode = 37;
pub(crate) const KEY_J: KeyCode = 38;
pub(crate) const KEY_K: KeyCode = 40;
pub(crate) const KEY_SEMICOLON: KeyCode = 41;

pub(crate) const KEY_LEFT_ARROW: KeyCode = 123;
pub(crate) const KEY_RIGHT_ARROW: KeyCode = 124;
pub(crate) const KEY_DOWN_ARROW: KeyCode = 125;
pub(crate) const KEY_UP_ARROW: KeyCode = 126;

/// `kCGEventFlagMaskCommand`.
pub(crate) const SHORTCUT_MODIFIER_FLAG_MASK: EventFlags = 1 << 20;
pub(crate) const SHORTCUT_MODIFIER_NAME: &str = "Command";
pub(crate) const PLATFORM_KEY_CODE_NAME: &str = "macOS key code";

/// Friendly names accepted for the layer key, paired with macOS virtual key
/// codes. Names are matched case- and separator-insensitively by `keymap`.
pub(crate) const LAYER_KEY_NAMES: &[(&str, KeyCode)] = &[
    ("semicolon", 41),
    ("quote", 39),
    ("apostrophe", 39),
    ("grave", 50),
    ("backtick", 50),
    ("tab", 48),
    ("return", 36),
    ("enter", 36),
    ("space", 49),
    ("escape", 53),
    ("delete", 51),
    ("backslash", 42),
    ("left_bracket", 33),
    ("right_bracket", 30),
    ("comma", 43),
    ("period", 47),
    ("slash", 44),
    ("minus", 27),
    ("equal", 24),
    // Modifier keys. Left/right are distinct physical keys with distinct
    // codes; both are usable as the layer key.
    ("left_command", 55),
    ("right_command", 54),
    ("left_option", 58),
    ("right_option", 61),
    ("left_alt", 58),
    ("right_alt", 61),
    ("left_control", 59),
    ("right_control", 62),
    ("left_ctrl", 59),
    ("right_ctrl", 62),
    ("left_shift", 56),
    ("right_shift", 60),
];

pub(crate) const MODIFIER_KEY_CODES: &[KeyCode] = &[55, 54, 56, 60, 59, 62, 58, 61];

/// Modifier key codes usable as the layer key, paired with the
/// device-dependent CGEvent flag bit that is set while that specific physical
/// key is held. The device bit distinguishes left from right (unlike the
/// general modifier masks) and lets macOS derive `flagsChanged` direction.
pub(crate) const MODIFIER_DEVICE_FLAGS: &[(KeyCode, EventFlags)] = &[
    (55, 0x0000_0008), // left command  (NX_DEVICELCMDKEYMASK)
    (54, 0x0000_0010), // right command (NX_DEVICERCMDKEYMASK)
    (56, 0x0000_0002), // left shift     (NX_DEVICELSHIFTKEYMASK)
    (60, 0x0000_0004), // right shift    (NX_DEVICERSHIFTKEYMASK)
    (59, 0x0000_0001), // left control   (NX_DEVICELCTLKEYMASK)
    (62, 0x0000_2000), // right control  (NX_DEVICERCTLKEYMASK)
    (58, 0x0000_0020), // left option    (NX_DEVICELALTKEYMASK)
    (61, 0x0000_0040), // right option   (NX_DEVICERALTKEYMASK)
];

pub(crate) const MODIFIER_CLEAR_MASKS: &[(KeyCode, EventFlags)] = &[
    (55, 0x0000_0008 | SHORTCUT_MODIFIER_FLAG_MASK),
    (54, 0x0000_0010 | SHORTCUT_MODIFIER_FLAG_MASK),
    (56, 0x0000_0002 | (1 << 17)),
    (60, 0x0000_0004 | (1 << 17)),
    (58, 0x0000_0020 | (1 << 19)),
    (61, 0x0000_0040 | (1 << 19)),
    (59, 0x0000_0001 | (1 << 18)),
    (62, 0x0000_2000 | (1 << 18)),
];

/// Caps Lock (57) and Fn (63) have special/toggle semantics that this tool
/// does not support as a layer key.
pub(crate) const UNSUPPORTED_MODIFIER_CODES: &[KeyCode] = &[57, 63];
