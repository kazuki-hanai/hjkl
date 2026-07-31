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
    K_CG_EVENT_FLAG_MASK_NUMERIC_PAD, K_CG_EVENT_FLAG_MASK_SECONDARY_FN,
    K_CG_EVENT_SOURCE_USER_DATA, K_CG_HID_EVENT_TAP, K_CG_KEYBOARD_EVENT_KEYCODE,
};

// Marker placed on events synthesized by this process. Without it, the
// event tap would see its own synthetic semicolon key events and suppress
// them again.
pub(crate) const SYNTHETIC_EVENT_TAG: i64 = 0x686A_6B6C_5F72_7374; // "hjkl_rst"

// Physical arrow-key events carry both of these macOS flags. In particular,
// kCGEventFlagMaskNumericPad identifies arrow/navigation keys to AppKit and to
// global-shortcut implementations that compare full modifier flags.
const ARROW_EVENT_FLAG_MASK: CGEventFlags =
    K_CG_EVENT_FLAG_MASK_NUMERIC_PAD | K_CG_EVENT_FLAG_MASK_SECONDARY_FN;

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

fn arrow_unicode(arrow_key: CGKeyCode) -> Option<u16> {
    match arrow_key {
        keymap::KEY_LEFT_ARROW => Some(0x001c),
        keymap::KEY_RIGHT_ARROW => Some(0x001d),
        keymap::KEY_DOWN_ARROW => Some(0x001f),
        keymap::KEY_UP_ARROW => Some(0x001e),
        _ => None,
    }
}

pub(crate) fn rewrite_as_arrow(event: CGEventRef, arrow_key: CGKeyCode) {
    unsafe {
        CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, i64::from(arrow_key));
        CGEventSetFlags(event, CGEventGetFlags(event) | ARROW_EVENT_FLAG_MASK);

        // Changing the key code does not replace the original h/j/k/l Unicode
        // payload. Use the same control character macOS assigns to a native
        // arrow event so consumers of either representation see an arrow.
        if let Some(unicode) = arrow_unicode(arrow_key) {
            CGEventKeyboardSetUnicodeString(event, 1, &unicode);
        }
    }
}

pub(crate) fn add_shortcut_modifier_flag(event: CGEventRef) {
    unsafe {
        CGEventSetFlags(
            event,
            keymap::with_shortcut_modifier_flag(CGEventGetFlags(event)),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macos::ffi::CGEventKeyboardGetUnicodeString;

    const CONTROL_FLAG_MASK: CGEventFlags = 1 << 18;

    #[test]
    fn arrow_unicode_matches_native_macos_events() {
        assert_eq!(arrow_unicode(keymap::KEY_LEFT_ARROW), Some(0x001c));
        assert_eq!(arrow_unicode(keymap::KEY_RIGHT_ARROW), Some(0x001d));
        assert_eq!(arrow_unicode(keymap::KEY_DOWN_ARROW), Some(0x001f));
        assert_eq!(arrow_unicode(keymap::KEY_UP_ARROW), Some(0x001e));
        assert_eq!(arrow_unicode(keymap::KEY_L), None);
    }

    #[test]
    fn rewrite_as_arrow_preserves_control_and_adds_native_arrow_metadata() {
        let event = unsafe { CGEventCreateKeyboardEvent(ptr::null(), keymap::KEY_L, true) };
        let native_arrow =
            unsafe { CGEventCreateKeyboardEvent(ptr::null(), keymap::KEY_RIGHT_ARROW, true) };
        assert!(!event.is_null());
        assert!(!native_arrow.is_null());

        unsafe {
            CGEventSetFlags(event, flags(event) | CONTROL_FLAG_MASK);
            CGEventSetFlags(native_arrow, flags(native_arrow) | CONTROL_FLAG_MASK);
        }

        rewrite_as_arrow(event, keymap::KEY_RIGHT_ARROW);

        assert_eq!(key_code(event), i64::from(keymap::KEY_RIGHT_ARROW));
        assert_eq!(flags(event) & CONTROL_FLAG_MASK, CONTROL_FLAG_MASK);
        assert_eq!(
            flags(event) & ARROW_EVENT_FLAG_MASK,
            flags(native_arrow) & ARROW_EVENT_FLAG_MASK
        );

        let mut actual_length = 0;
        let mut unicode = 0;
        unsafe {
            CGEventKeyboardGetUnicodeString(event, 1, &mut actual_length, &mut unicode);
        }
        assert_eq!(actual_length, 1);
        assert_eq!(unicode, 0x001d);

        unsafe {
            CFRelease(event.cast());
            CFRelease(native_arrow.cast());
        }
    }
}
