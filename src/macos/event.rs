//! Helpers around `CGEventRef`: reading and rewriting keyboard events, and
//! posting synthetic ones.
//!
//! Callers must pass event references obtained from the tap callback (never
//! null); the callback checks for null before reaching this module.

use std::ptr;

use crate::keymap;
use crate::macos::ffi::{
    CFRelease, CGEventCreateKeyboardEvent, CGEventFlags, CGEventGetFlags,
    CGEventGetIntegerValueField, CGEventKeyboardSetUnicodeString, CGEventMask, CGEventPost,
    CGEventRef, CGEventSetFlags, CGEventSetIntegerValueField, CGEventType, CGKeyCode,
    K_CG_EVENT_SOURCE_USER_DATA, K_CG_HID_EVENT_TAP, K_CG_KEYBOARD_EVENT_KEYCODE,
};

// Marker placed on events synthesized by this process. Without it, the
// event tap would see its own synthetic semicolon key events and suppress
// them again.
pub(crate) const SYNTHETIC_EVENT_TAG: i64 = 0x686A_6B6C_5F72_7374; // "hjkl_rst"

pub(crate) fn event_mask(event_type: CGEventType) -> CGEventMask {
    1u64 << event_type
}

/// Returned from the tap callback to swallow the current event.
pub(crate) fn suppress() -> CGEventRef {
    ptr::null_mut()
}

pub(crate) fn user_data(event: CGEventRef) -> i64 {
    unsafe { CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) }
}

pub(crate) fn key_code(event: CGEventRef) -> i64 {
    unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) }
}

pub(crate) fn flags(event: CGEventRef) -> CGEventFlags {
    unsafe { CGEventGetFlags(event) }
}

pub(crate) fn rewrite_as_arrow(event: CGEventRef, arrow_key: CGKeyCode) {
    unsafe {
        CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, i64::from(arrow_key));
        // The original event still has the text payload for h/j/k/l. Clear
        // it so apps that inspect Unicode see a real non-text arrow key
        // event.
        CGEventKeyboardSetUnicodeString(event, 0, ptr::null());
    }
}

pub(crate) fn add_command_flag(event: CGEventRef) {
    unsafe {
        CGEventSetFlags(event, keymap::with_command_flag(CGEventGetFlags(event)));
    }
}

/// Clear the given modifier flag bits from the event. Used when a modifier key
/// serves as the layer key, so its still-held physical flag does not ride on
/// the arrow / command events the layer produces.
pub(crate) fn clear_flags(event: CGEventRef, mask: CGEventFlags) {
    unsafe {
        CGEventSetFlags(event, CGEventGetFlags(event) & !mask);
    }
}

/// Post a full key press (down + up) as this process, tagged so our own tap
/// ignores it.
pub(crate) fn post_key(key_code: CGKeyCode, flags: CGEventFlags) {
    post_keyboard_event(key_code, true, flags);
    post_keyboard_event(key_code, false, flags);
}

fn post_keyboard_event(key_code: CGKeyCode, key_down: bool, flags: CGEventFlags) {
    let event = unsafe { CGEventCreateKeyboardEvent(ptr::null(), key_code, key_down) };
    if event.is_null() {
        return;
    }

    unsafe {
        CGEventSetFlags(event, flags);
        CGEventSetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA, SYNTHETIC_EVENT_TAG);
        CGEventPost(K_CG_HID_EVENT_TAP, event);
        CFRelease(event.cast());
    }
}
