//! Helpers around `CGEventRef`: reading and rewriting keyboard events, and
//! posting synthetic ones.
//!
//! Callers must pass event references obtained from the tap callback (never
//! null); the callback checks for null before reaching this module.

use std::ptr;

use crate::keymap;
use crate::macos::ffi::{
    CFRelease, CGEventCreateKeyboardEvent, CGEventCreateSourceFromEvent, CGEventFlags,
    CGEventGetFlags, CGEventGetIntegerValueField, CGEventGetTimestamp, CGEventGetType,
    CGEventKeyboardSetUnicodeString, CGEventMask, CGEventPost, CGEventRef, CGEventSetFlags,
    CGEventSetIntegerValueField, CGEventSetTimestamp, CGEventType, CGKeyCode,
    K_CG_EVENT_FLAG_MASK_NUMERIC_PAD, K_CG_EVENT_FLAG_MASK_SECONDARY_FN, K_CG_EVENT_KEY_DOWN,
    K_CG_EVENT_SOURCE_USER_DATA, K_CG_HID_EVENT_TAP, K_CG_KEYBOARD_EVENT_AUTOREPEAT,
    K_CG_KEYBOARD_EVENT_KEYBOARD_TYPE, K_CG_KEYBOARD_EVENT_KEYCODE,
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

/// Turn an `h`/`j`/`k`/`l` key event into the matching arrow key.
///
/// Returns the event the tap callback should deliver in place of the incoming
/// one. Instead of editing the fields of the original letter event, this builds
/// a brand-new arrow key event so the result is byte-for-byte identical to a
/// physically pressed arrow key.
///
/// This matters beyond the visible key code: `CGEventSetIntegerValueField` only
/// swaps the key code and leaves the event's internal character fields set to
/// the original `h`/`j`/`k`/`l`. AppKit and, in particular, IME composition read
/// those fields, so an in-place rewrite is treated as a letter mid-conversion
/// instead of an arrow, breaking `; + h/j/k/l` navigation while composing text.
/// A freshly created event carries the correct arrow character fields, the
/// `numericPad`/`secondaryFn` flags physical arrows have, and any modifiers held
/// at press time (so `Control + ; + l` still behaves like `Control + Right
/// Arrow`).
///
/// If a replacement event cannot be allocated, the original event is rewritten
/// in place as a fallback and returned, so a keystroke is never dropped.
#[must_use]
pub(crate) fn rewrite_as_arrow(event: CGEventRef, arrow_key: CGKeyCode) -> CGEventRef {
    unsafe {
        let key_down = CGEventGetType(event) == K_CG_EVENT_KEY_DOWN;

        let source = CGEventCreateSourceFromEvent(event);
        let arrow = CGEventCreateKeyboardEvent(source, arrow_key, key_down);
        if !source.is_null() {
            CFRelease(source.cast());
        }

        if arrow.is_null() {
            rewrite_in_place(event, arrow_key);
            return event;
        }

        // Preserve modifiers held at press time and mark the event as a
        // navigation key, exactly as a physical arrow press would be.
        CGEventSetFlags(arrow, CGEventGetFlags(event) | ARROW_EVENT_FLAG_MASK);
        // Carry over timing, auto-repeat and keyboard type so the arrow is
        // indistinguishable from the key press it replaces.
        CGEventSetTimestamp(arrow, CGEventGetTimestamp(event));
        CGEventSetIntegerValueField(
            arrow,
            K_CG_KEYBOARD_EVENT_AUTOREPEAT,
            CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_AUTOREPEAT),
        );
        CGEventSetIntegerValueField(
            arrow,
            K_CG_KEYBOARD_EVENT_KEYBOARD_TYPE,
            CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYBOARD_TYPE),
        );
        arrow
    }
}

/// Fallback for the rare case where a replacement event cannot be created:
/// mutate the original letter event in place. Consumers that read the letter's
/// Unicode payload (notably IME composition) may misbehave, but the keystroke
/// is not lost.
fn rewrite_in_place(event: CGEventRef, arrow_key: CGKeyCode) {
    unsafe {
        CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, i64::from(arrow_key));
        CGEventSetFlags(event, CGEventGetFlags(event) | ARROW_EVENT_FLAG_MASK);
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
    use crate::macos::ffi::{
        CFDataGetBytePtr, CFDataGetLength, CGEventCreateData, CGEventKeyboardGetUnicodeString,
    };

    const CONTROL_FLAG_MASK: CGEventFlags = 1 << 18;

    fn unicode_payload(event: CGEventRef) -> Vec<u16> {
        let mut buf = [0u16; 8];
        let mut len: u64 = 0;
        unsafe {
            CGEventKeyboardGetUnicodeString(event, buf.len() as u64, &mut len, buf.as_mut_ptr());
        }
        buf[..len as usize].to_vec()
    }

    /// Compare two events by their flattened ("serialized") representation,
    /// which includes the internal character fields a plain key-code swap leaves
    /// pointing at the original letter.
    fn flattened_eq(a: CGEventRef, b: CGEventRef) -> bool {
        unsafe {
            let data_a = CGEventCreateData(ptr::null(), a);
            let data_b = CGEventCreateData(ptr::null(), b);
            assert!(!data_a.is_null() && !data_b.is_null());
            let len_a = CFDataGetLength(data_a) as usize;
            let len_b = CFDataGetLength(data_b) as usize;
            let equal = len_a == len_b
                && std::slice::from_raw_parts(CFDataGetBytePtr(data_a), len_a)
                    == std::slice::from_raw_parts(CFDataGetBytePtr(data_b), len_b);
            CFRelease(data_a.cast());
            CFRelease(data_b.cast());
            equal
        }
    }

    #[test]
    fn arrow_unicode_matches_native_macos_events() {
        assert_eq!(arrow_unicode(keymap::KEY_LEFT_ARROW), Some(0x001c));
        assert_eq!(arrow_unicode(keymap::KEY_RIGHT_ARROW), Some(0x001d));
        assert_eq!(arrow_unicode(keymap::KEY_DOWN_ARROW), Some(0x001f));
        assert_eq!(arrow_unicode(keymap::KEY_UP_ARROW), Some(0x001e));
        assert_eq!(arrow_unicode(keymap::KEY_L), None);
    }

    /// The rewritten event must be indistinguishable from a physical arrow key,
    /// down to the internal character fields IME composition reads. The old
    /// in-place rewrite left those fields set to h/j/k/l, so arrows worked in
    /// plain text fields but broke `; + h/j/k/l` navigation while composing
    /// Japanese text -- even though the key code and Unicode string looked right.
    fn assert_matches_native_arrow(letter_key: CGKeyCode, arrow_key: CGKeyCode, key_down: bool) {
        unsafe {
            let letter = CGEventCreateKeyboardEvent(ptr::null(), letter_key, key_down);
            let native = CGEventCreateKeyboardEvent(ptr::null(), arrow_key, key_down);
            assert!(!letter.is_null() && !native.is_null());

            // Simulate `Control + ; + <letter>` and pin timestamps so only the
            // semantic bytes are compared.
            CGEventSetFlags(letter, flags(letter) | CONTROL_FLAG_MASK);
            CGEventSetFlags(native, flags(native) | CONTROL_FLAG_MASK);
            CGEventSetTimestamp(letter, 42);
            CGEventSetTimestamp(native, 42);

            let produced = rewrite_as_arrow(letter, arrow_key);
            assert!(!produced.is_null());
            assert_ne!(produced, letter, "a fresh event should replace the letter");

            assert_eq!(key_code(produced), i64::from(arrow_key));
            assert_eq!(flags(produced) & CONTROL_FLAG_MASK, CONTROL_FLAG_MASK);
            assert_eq!(
                flags(produced) & ARROW_EVENT_FLAG_MASK,
                ARROW_EVENT_FLAG_MASK
            );
            assert!(
                flattened_eq(produced, native),
                "rewritten arrow must be byte-identical to a native arrow event"
            );

            CFRelease(produced.cast());
            CFRelease(native.cast());
            CFRelease(letter.cast());
        }
    }

    #[test]
    fn rewrite_as_arrow_is_byte_identical_to_native_arrow_on_key_down() {
        assert_matches_native_arrow(keymap::KEY_H, keymap::KEY_LEFT_ARROW, true);
        assert_matches_native_arrow(keymap::KEY_J, keymap::KEY_DOWN_ARROW, true);
        assert_matches_native_arrow(keymap::KEY_K, keymap::KEY_UP_ARROW, true);
        assert_matches_native_arrow(keymap::KEY_L, keymap::KEY_RIGHT_ARROW, true);
    }

    #[test]
    fn rewrite_as_arrow_is_byte_identical_to_native_arrow_on_key_up() {
        // Key-up must also match, or a held arrow could get stuck down.
        assert_matches_native_arrow(keymap::KEY_L, keymap::KEY_RIGHT_ARROW, false);
    }

    #[test]
    fn rewrite_as_arrow_preserves_autorepeat_and_matches_native_arrow() {
        unsafe {
            let letter = CGEventCreateKeyboardEvent(ptr::null(), keymap::KEY_L, true);
            let native = CGEventCreateKeyboardEvent(ptr::null(), keymap::KEY_RIGHT_ARROW, true);
            assert!(!letter.is_null() && !native.is_null());

            CGEventSetTimestamp(letter, 42);
            CGEventSetTimestamp(native, 42);
            CGEventSetIntegerValueField(letter, K_CG_KEYBOARD_EVENT_AUTOREPEAT, 1);
            CGEventSetIntegerValueField(native, K_CG_KEYBOARD_EVENT_AUTOREPEAT, 1);

            let produced = rewrite_as_arrow(letter, keymap::KEY_RIGHT_ARROW);
            assert_eq!(
                CGEventGetIntegerValueField(produced, K_CG_KEYBOARD_EVENT_AUTOREPEAT),
                1
            );
            assert!(
                flattened_eq(produced, native),
                "an auto-repeated arrow must match a native repeated arrow event"
            );

            CFRelease(produced.cast());
            CFRelease(native.cast());
            CFRelease(letter.cast());
        }
    }

    #[test]
    fn rewrite_as_arrow_delivers_the_arrow_character_not_the_letter() {
        use crate::macos::ffi::CGKeyCode;

        let cases: [(CGKeyCode, CGKeyCode); 4] = [
            (keymap::KEY_H, keymap::KEY_LEFT_ARROW),
            (keymap::KEY_J, keymap::KEY_DOWN_ARROW),
            (keymap::KEY_K, keymap::KEY_UP_ARROW),
            (keymap::KEY_L, keymap::KEY_RIGHT_ARROW),
        ];

        for (letter_key, arrow_key) in cases {
            unsafe {
                let letter = CGEventCreateKeyboardEvent(ptr::null(), letter_key, true);
                let native = CGEventCreateKeyboardEvent(ptr::null(), arrow_key, true);
                assert!(!letter.is_null() && !native.is_null());

                let letter_payload = unicode_payload(letter);
                let produced = rewrite_as_arrow(letter, arrow_key);
                assert!(!produced.is_null());

                // The delivered event carries exactly what a real arrow key
                // carries -- and therefore not the original letter. (Compared
                // against a live arrow so the assertion holds regardless of how
                // the host materializes an event's Unicode payload.)
                assert_eq!(unicode_payload(produced), unicode_payload(native));
                if !letter_payload.is_empty() {
                    assert_ne!(unicode_payload(produced), letter_payload);
                }

                CFRelease(produced.cast());
                CFRelease(native.cast());
                if produced != letter {
                    CFRelease(letter.cast());
                }
            }
        }
    }
}
