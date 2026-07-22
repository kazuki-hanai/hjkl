//! Narrow wrappers around `SendInput`.

use std::mem;

use crate::keymap::{KeyCode, KeyDirection};
use crate::windows::keys;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput,
    VK_DELETE, VK_DOWN, VK_END, VK_HOME, VK_INSERT, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RCONTROL,
    VK_RIGHT, VK_RMENU, VK_UP,
};

/// Marker placed on events synthesized by this process. Without it, the
/// low-level hook would see its own events and process them again.
pub(crate) const SYNTHETIC_EVENT_TAG: usize = 0x686A_6B6C_5F72_7374; // "hjkl_rst"

pub(crate) fn post_key(key_code: KeyCode) {
    send_key_event(key_code, KeyDirection::Down);
    send_key_event(key_code, KeyDirection::Up);
}

pub(crate) fn press_shortcut_modifier() {
    send_key_event(keys::SHORTCUT_MODIFIER_KEY, KeyDirection::Down);
}

pub(crate) fn release_shortcut_modifier() {
    send_key_event(keys::SHORTCUT_MODIFIER_KEY, KeyDirection::Up);
}

pub(crate) fn send_key_event(key_code: KeyCode, direction: KeyDirection) {
    let mut flags = if direction == KeyDirection::Up {
        KEYEVENTF_KEYUP
    } else {
        0
    };

    if is_extended_key(key_code) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let inputs = [INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key_code,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SYNTHETIC_EVENT_TAG,
            },
        },
    }];

    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            mem::size_of::<INPUT>() as i32,
        );
    }
}

fn is_extended_key(key_code: KeyCode) -> bool {
    matches!(
        key_code,
        VK_LEFT
            | VK_UP
            | VK_RIGHT
            | VK_DOWN
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_INSERT
            | VK_DELETE
            | VK_RCONTROL
            | VK_RMENU
    )
}
