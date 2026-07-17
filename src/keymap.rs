//! Pure semicolon-layer mapping logic.
//!
//! Everything here is plain data and arithmetic — no FFI — so it can be
//! tested without macOS Accessibility permissions.

/// macOS virtual key code (same representation as `CGKeyCode`).
pub(crate) type KeyCode = u16;

/// Keyboard event modifier flags (same representation as `CGEventFlags`).
pub(crate) type EventFlags = u64;

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
pub(crate) const COMMAND_FLAG_MASK: EventFlags = 1 << 20;

pub(crate) fn hjkl_to_arrow(key_code: KeyCode) -> Option<KeyCode> {
    match key_code {
        KEY_H => Some(KEY_LEFT_ARROW),
        KEY_J => Some(KEY_DOWN_ARROW),
        KEY_K => Some(KEY_UP_ARROW),
        KEY_L => Some(KEY_RIGHT_ARROW),
        _ => None,
    }
}

/// Distinct state bit for each layer-mapped key, used to keep tracking a
/// key that went down while the layer was active until it comes back up.
pub(crate) fn hjkl_key_bit(key_code: KeyCode) -> Option<u8> {
    match key_code {
        KEY_H => Some(1 << 0),
        KEY_J => Some(1 << 1),
        KEY_K => Some(1 << 2),
        KEY_L => Some(1 << 3),
        _ => None,
    }
}

pub(crate) fn with_command_flag(flags: EventFlags) -> EventFlags {
    flags | COMMAND_FLAG_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_hjkl_to_arrow_keys() {
        assert_eq!(hjkl_to_arrow(KEY_H), Some(KEY_LEFT_ARROW));
        assert_eq!(hjkl_to_arrow(KEY_J), Some(KEY_DOWN_ARROW));
        assert_eq!(hjkl_to_arrow(KEY_K), Some(KEY_UP_ARROW));
        assert_eq!(hjkl_to_arrow(KEY_L), Some(KEY_RIGHT_ARROW));
    }

    #[test]
    fn does_not_map_unrelated_keys() {
        assert_eq!(hjkl_to_arrow(KEY_SEMICOLON), None);
        assert_eq!(hjkl_to_arrow(0), None); // A
    }

    #[test]
    fn maps_hjkl_keys_to_distinct_state_bits() {
        assert_eq!(hjkl_key_bit(KEY_H), Some(1 << 0));
        assert_eq!(hjkl_key_bit(KEY_J), Some(1 << 1));
        assert_eq!(hjkl_key_bit(KEY_K), Some(1 << 2));
        assert_eq!(hjkl_key_bit(KEY_L), Some(1 << 3));
        assert_eq!(hjkl_key_bit(KEY_SEMICOLON), None);
    }

    #[test]
    fn command_flag_is_added_without_dropping_other_flags() {
        let shift_flag: EventFlags = 1 << 17;
        assert_eq!(
            with_command_flag(shift_flag),
            shift_flag | COMMAND_FLAG_MASK
        );
        assert_eq!(
            with_command_flag(shift_flag | COMMAND_FLAG_MASK),
            shift_flag | COMMAND_FLAG_MASK
        );
    }
}
