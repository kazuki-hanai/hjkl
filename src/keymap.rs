//! Pure layer-key mapping logic and state machine.
//!
//! Everything here is plain data and arithmetic -- no FFI -- so the full layer
//! behavior can be tested without platform permissions. The macOS event tap and
//! Windows keyboard hook translate each keyboard event into a
//! [`LayerState::on_key`] call and perform the returned [`Action`].

#![cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]

use crate::platform::keys;

/// Platform virtual key code (`CGKeyCode` on macOS, `VIRTUAL_KEY` on Windows).
pub(crate) type KeyCode = u16;

/// Platform keyboard modifier flags. macOS uses `CGEventFlags`; Windows does
/// not rewrite in-place and therefore only carries zero here.
pub(crate) type EventFlags = u64;

pub(crate) use keys::{
    KEY_DOWN_ARROW, KEY_H, KEY_J, KEY_K, KEY_L, KEY_LEFT_ARROW, KEY_RIGHT_ARROW, KEY_SEMICOLON,
    KEY_UP_ARROW,
};

/// The layer (a.k.a. "super") key used when none is configured.
pub(crate) const DEFAULT_LAYER_KEY: KeyCode = KEY_SEMICOLON;

fn normalize_key_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '-' && *c != ' ')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether a key code is a modifier the layer machine drives via
/// platform-specific modifier events. A modifier used as the layer key is not
/// replayed when tapped on its own.
pub(crate) fn is_modifier(key_code: KeyCode) -> bool {
    keys::MODIFIER_KEY_CODES.contains(&key_code)
}

/// The device-dependent flag bit set while the given modifier key is held, if
/// it is a supported modifier. Only macOS needs this because modifier key
/// transitions arrive as `flagsChanged` rather than key-down/up events.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn modifier_device_flag(key_code: KeyCode) -> Option<EventFlags> {
    keys::MODIFIER_DEVICE_FLAGS
        .iter()
        .find(|(code, _)| *code == key_code)
        .map(|(_, mask)| *mask)
}

/// All flag bits (general mask + device bit) contributed by holding the given
/// modifier key. When the modifier is the layer key, these are stripped from
/// the events the layer emits so, e.g., `right_command + j` produces a bare
/// Down arrow rather than Command+Down.
///
/// Limitation: the general mask (e.g. the command flag on macOS) is shared by the
/// left and right key of the same modifier. If the user genuinely holds the
/// opposite-side key of the same modifier while the layer is active, clearing
/// the general bit also drops that key's contribution. This is a rare edge and
/// accepted; the device bit still distinguishes the two physically.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn modifier_clear_mask(key_code: KeyCode) -> Option<EventFlags> {
    keys::MODIFIER_CLEAR_MASKS
        .iter()
        .find(|(code, _)| *code == key_code)
        .map(|(_, mask)| *mask)
}

/// Parse a user-supplied layer-key spec — either a friendly name
/// (`semicolon`, `quote`, `right_command` …, case- and separator-insensitive)
/// or a raw decimal platform virtual key code — into a validated key code.
pub(crate) fn parse_layer_key(spec: &str) -> Result<KeyCode, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("layer key is empty".to_string());
    }

    let normalized = normalize_key_name(trimmed);

    let key_code = if let Some((_, code)) = keys::LAYER_KEY_NAMES
        .iter()
        .find(|(name, _)| normalize_key_name(name) == normalized)
    {
        *code
    } else if let Ok(code) = normalized.parse::<KeyCode>() {
        code
    } else {
        return Err(format!(
            "unknown layer key '{spec}'. Use a name like 'semicolon', 'quote', or \
             'right_command', or a numeric {}.",
            keys::PLATFORM_KEY_CODE_NAME
        ));
    };

    validate_layer_key(key_code)?;
    Ok(key_code)
}

fn validate_layer_key(key_code: KeyCode) -> Result<(), String> {
    if keys::UNSUPPORTED_MODIFIER_CODES.contains(&key_code) {
        return Err(format!(
            "key code {key_code} is not supported as the layer key because it has special/toggle behavior."
        ));
    }
    if hjkl_key_bit(key_code).is_some() {
        return Err(format!(
            "key code {key_code} is one of h/j/k/l, which the layer maps to arrow \
             keys and so cannot also be the layer key."
        ));
    }
    if matches!(
        key_code,
        KEY_LEFT_ARROW | KEY_RIGHT_ARROW | KEY_DOWN_ARROW | KEY_UP_ARROW
    ) {
        return Err(format!(
            "key code {key_code} is an arrow key that the layer emits, so it \
             cannot also be the layer key."
        ));
    }
    Ok(())
}

/// Canonical display name for a key code, if it has one.
pub(crate) fn layer_key_name(key_code: KeyCode) -> Option<&'static str> {
    keys::LAYER_KEY_NAMES
        .iter()
        .find(|(_, code)| *code == key_code)
        .map(|(name, _)| *name)
}

#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub(crate) fn layer_key_label(key_code: KeyCode) -> String {
    match layer_key_name(key_code) {
        Some(name) => format!("{name} (keycode {key_code})"),
        None => format!("keycode {key_code}"),
    }
}

pub(crate) fn shortcut_modifier_name() -> &'static str {
    keys::SHORTCUT_MODIFIER_NAME
}

pub(crate) fn platform_key_code_name() -> &'static str {
    keys::PLATFORM_KEY_CODE_NAME
}

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

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn with_shortcut_modifier_flag(flags: EventFlags) -> EventFlags {
    flags | keys::SHORTCUT_MODIFIER_FLAG_MASK
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyDirection {
    Down,
    Up,
}

/// What the event tap should do with the current keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Deliver the event unchanged.
    PassThrough,
    /// Swallow the event.
    Suppress,
    /// Rewrite the event in place as the given arrow key, then deliver it.
    RewriteArrow(KeyCode),
    /// Post a synthetic press of the layer key with the captured modifier
    /// flags, and swallow the original event.
    PostLayerKeyAndSuppress(EventFlags),
    /// Add the platform shortcut modifier to the event, then deliver it.
    AddShortcutModifier,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerState {
    layer_down: bool,
    used_as_layer: bool,
    command_layer_active: bool,
    layer_flags: EventFlags,
    mapped_keys_down: u8,
}

impl LayerState {
    pub(crate) const fn new() -> Self {
        Self {
            layer_down: false,
            used_as_layer: false,
            command_layer_active: false,
            layer_flags: 0,
            mapped_keys_down: 0,
        }
    }

    fn clear_layer(&mut self) {
        self.layer_down = false;
        self.used_as_layer = false;
        self.command_layer_active = false;
        self.layer_flags = 0;
    }

    /// Advance the state machine by one keyboard event and return the action
    /// to perform. `layer_key` is the configured "super" key. `flags` are the
    /// event's modifier flags; they are only consulted when the layer key goes
    /// down, so the delayed layer key can be replayed with the modifiers that
    /// were held at press time.
    pub(crate) fn on_key(
        &mut self,
        layer_key: KeyCode,
        direction: KeyDirection,
        key_code: KeyCode,
        flags: EventFlags,
    ) -> Action {
        if let Some(key_bit) = hjkl_key_bit(key_code)
            && self.mapped_keys_down & key_bit != 0
        {
            if direction == KeyDirection::Up {
                self.mapped_keys_down &= !key_bit;
            }

            if let Some(arrow_key) = hjkl_to_arrow(key_code) {
                return Action::RewriteArrow(arrow_key);
            }
        }

        match (direction, key_code) {
            (KeyDirection::Down, k) if k == layer_key => {
                // Delay the layer key until key-up. If another key is pressed
                // in between, it became the layer key and should not be
                // emitted as text.
                if !self.layer_down {
                    self.layer_down = true;
                    self.used_as_layer = false;
                    self.layer_flags = flags;
                }
                Action::Suppress
            }
            (KeyDirection::Up, k) if k == layer_key && self.layer_down => {
                let should_post_layer_key = !self.used_as_layer;
                let layer_flags = self.layer_flags;

                self.clear_layer();

                if should_post_layer_key {
                    Action::PostLayerKeyAndSuppress(layer_flags)
                } else {
                    Action::Suppress
                }
            }
            (direction, key_code) if self.layer_down => {
                if let Some(arrow_key) = hjkl_to_arrow(key_code) {
                    if direction == KeyDirection::Down {
                        self.used_as_layer = true;
                        if let Some(key_bit) = hjkl_key_bit(key_code) {
                            self.mapped_keys_down |= key_bit;
                        }
                        Action::RewriteArrow(arrow_key)
                    } else {
                        Action::PassThrough
                    }
                } else {
                    // Karabiner's first rule turns the layer key into a
                    // command-like modifier when it is used with any other
                    // key. We emulate that for normal shortcuts by adding the
                    // platform shortcut modifier to non-hjkl events while the
                    // layer is held. hjkl is handled above and intentionally
                    // becomes a plain arrow key instead.
                    if direction == KeyDirection::Down {
                        self.used_as_layer = true;
                        self.command_layer_active = true;
                    }
                    if self.command_layer_active {
                        Action::AddShortcutModifier
                    } else {
                        Action::PassThrough
                    }
                }
            }
            _ => Action::PassThrough,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Action::*;
    use super::KeyDirection::{Down, Up};
    use super::*;

    const KEY_A: KeyCode = 0;
    const NO_FLAGS: EventFlags = 0;
    const SHIFT_FLAG: EventFlags = 1 << 17;
    // The layer key the state-machine tests drive with, unless a test needs a
    // different one. Kept as the historical default so the assertions read the
    // same as before layer-key configurability.
    const LAYER: KeyCode = KEY_SEMICOLON;

    fn named_key(name: &str) -> KeyCode {
        let normalized = normalize_key_name(name);
        keys::LAYER_KEY_NAMES
            .iter()
            .find(|(candidate, _)| normalize_key_name(candidate) == normalized)
            .map(|(_, code)| *code)
            .expect("test key name should exist on this platform")
    }

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
        assert_eq!(hjkl_to_arrow(KEY_A), None);
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
    fn shortcut_modifier_flag_is_added_without_dropping_other_flags() {
        let shortcut_flag = keys::SHORTCUT_MODIFIER_FLAG_MASK;
        assert_eq!(
            with_shortcut_modifier_flag(SHIFT_FLAG),
            SHIFT_FLAG | shortcut_flag
        );
        assert_eq!(
            with_shortcut_modifier_flag(SHIFT_FLAG | shortcut_flag),
            SHIFT_FLAG | shortcut_flag
        );
    }

    #[test]
    fn layer_key_tapped_alone_is_replayed_on_key_up() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Up, LAYER, NO_FLAGS),
            PostLayerKeyAndSuppress(NO_FLAGS)
        );
    }

    #[test]
    fn layer_key_replay_keeps_modifiers_held_at_press_time() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, SHIFT_FLAG), Suppress);
        // Modifiers at key-up time do not matter; press-time flags win.
        assert_eq!(
            state.on_key(LAYER, Up, LAYER, NO_FLAGS),
            PostLayerKeyAndSuppress(SHIFT_FLAG)
        );
    }

    #[test]
    fn layer_key_repeat_stays_suppressed_without_resetting_capture() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, SHIFT_FLAG), Suppress);
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Up, LAYER, NO_FLAGS),
            PostLayerKeyAndSuppress(SHIFT_FLAG)
        );
    }

    #[test]
    fn holding_layer_turns_hjkl_into_arrows_and_swallows_layer_key() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Down, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(
            state.on_key(LAYER, Up, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        // The layer key was used as a modifier, so it is not emitted.
        assert_eq!(state.on_key(LAYER, Up, LAYER, NO_FLAGS), Suppress);
    }

    #[test]
    fn arrow_key_up_is_still_rewritten_after_layer_release() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Down, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        // Layer key released while j is still held.
        assert_eq!(state.on_key(LAYER, Up, LAYER, NO_FLAGS), Suppress);
        // The j key-up must still become a down-arrow key-up, otherwise the
        // arrow key would be stuck down.
        assert_eq!(
            state.on_key(LAYER, Up, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        // With the layer gone, j is a plain key again.
        assert_eq!(state.on_key(LAYER, Down, KEY_J, NO_FLAGS), PassThrough);
    }

    #[test]
    fn key_repeat_of_mapped_key_keeps_rewriting_while_held() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Down, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
        // Auto-repeat key-downs while held keep being rewritten.
        assert_eq!(
            state.on_key(LAYER, Down, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
        assert_eq!(
            state.on_key(LAYER, Up, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
    }

    #[test]
    fn holding_layer_adds_shortcut_modifier_to_other_keys() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(LAYER, Down, KEY_A, NO_FLAGS),
            AddShortcutModifier
        );
        assert_eq!(
            state.on_key(LAYER, Up, KEY_A, NO_FLAGS),
            AddShortcutModifier
        );
        // The layer key acted as a modifier, so it is not emitted.
        assert_eq!(state.on_key(LAYER, Up, LAYER, NO_FLAGS), Suppress);
        // After release the layer is fully reset.
        assert_eq!(state.on_key(LAYER, Down, KEY_A, NO_FLAGS), PassThrough);
    }

    #[test]
    fn hjkl_key_up_without_prior_layer_press_passes_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        // h was pressed before the layer was activated, so its key-up is not
        // a mapped key and must pass through unchanged.
        assert_eq!(state.on_key(LAYER, Up, KEY_H, NO_FLAGS), PassThrough);
    }

    #[test]
    fn layer_key_up_without_key_down_passes_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Up, LAYER, NO_FLAGS), PassThrough);
    }

    #[test]
    fn keys_without_layer_pass_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, KEY_A, NO_FLAGS), PassThrough);
        assert_eq!(state.on_key(LAYER, Down, KEY_H, NO_FLAGS), PassThrough);
        assert_eq!(state.on_key(LAYER, Up, KEY_H, NO_FLAGS), PassThrough);
    }

    #[test]
    fn a_custom_layer_key_drives_the_layer_and_semicolon_types_normally() {
        let quote = named_key("quote");
        let mut state = LayerState::new();

        // Semicolon is now just a normal key and passes through.
        assert_eq!(
            state.on_key(quote, Down, KEY_SEMICOLON, NO_FLAGS),
            PassThrough
        );

        // Quote drives the layer: hold quote + j -> down arrow.
        assert_eq!(state.on_key(quote, Down, quote, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(quote, Down, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        assert_eq!(
            state.on_key(quote, Up, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        assert_eq!(state.on_key(quote, Up, quote, NO_FLAGS), Suppress);

        // Tapping quote alone replays quote.
        assert_eq!(state.on_key(quote, Down, quote, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(quote, Up, quote, NO_FLAGS),
            PostLayerKeyAndSuppress(NO_FLAGS)
        );
    }

    #[test]
    fn parses_layer_key_names_and_codes() {
        assert_eq!(parse_layer_key("semicolon"), Ok(KEY_SEMICOLON));
        assert_eq!(parse_layer_key("quote"), Ok(named_key("quote")));
        assert_eq!(parse_layer_key("apostrophe"), Ok(named_key("quote")));
        // Case- and separator-insensitive.
        assert_eq!(
            parse_layer_key("Right-Bracket"),
            Ok(named_key("right_bracket"))
        );
        assert_eq!(parse_layer_key("  TAB  "), Ok(named_key("tab")));
        // Raw numeric codes.
        let quote = named_key("quote");
        let grave = named_key("grave");
        assert_eq!(parse_layer_key(&quote.to_string()), Ok(quote));
        assert_eq!(parse_layer_key(&grave.to_string()), Ok(grave));
    }

    #[test]
    fn parses_modifier_layer_keys_with_left_right() {
        let right_command = named_key("right_command");
        let left_command = named_key("left_command");
        assert_eq!(parse_layer_key("right_command"), Ok(right_command));
        assert_eq!(parse_layer_key("left_command"), Ok(left_command));
        assert_eq!(
            parse_layer_key("Right Option"),
            Ok(named_key("right_option"))
        );
        assert_eq!(parse_layer_key("left-ctrl"), Ok(named_key("left_control")));
        assert_eq!(parse_layer_key("right_shift"), Ok(named_key("right_shift")));
        // Numeric modifier codes are accepted too.
        assert_eq!(
            parse_layer_key(&right_command.to_string()),
            Ok(right_command)
        );

        assert!(is_modifier(right_command));
        assert!(is_modifier(left_command));
        assert!(!is_modifier(KEY_SEMICOLON)); // semicolon is not a modifier
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_modifier_device_flags_track_left_and_right_keys() {
        let left_command = named_key("left_command");
        let right_command = named_key("right_command");
        // Left and right of the same modifier get distinct device flag bits.
        assert_eq!(modifier_device_flag(left_command), Some(0x8));
        assert_eq!(modifier_device_flag(right_command), Some(0x10));
        assert_ne!(
            modifier_device_flag(left_command),
            modifier_device_flag(right_command)
        );
        assert_eq!(modifier_device_flag(KEY_SEMICOLON), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_modifier_clear_mask_includes_device_and_general_bits() {
        let right_command = named_key("right_command");
        let left_shift = named_key("left_shift");
        // right command: device bit 0x10 + general command mask (1 << 20).
        assert_eq!(
            modifier_clear_mask(right_command),
            Some(0x10 | keys::SHORTCUT_MODIFIER_FLAG_MASK)
        );
        // left shift: device bit 0x2 + general shift mask (1 << 17).
        assert_eq!(modifier_clear_mask(left_shift), Some(0x2 | (1 << 17)));
        // Non-modifier keys contribute no mask.
        assert_eq!(modifier_clear_mask(KEY_SEMICOLON), None);
    }

    #[test]
    fn rejects_invalid_layer_keys() {
        assert!(parse_layer_key("").is_err());
        assert!(parse_layer_key("not_a_key").is_err());
        if let Some(code) = keys::UNSUPPORTED_MODIFIER_CODES.first() {
            assert!(parse_layer_key(&code.to_string()).is_err());
        }
        // h/j/k/l are arrow targets and cannot also be the layer key.
        assert!(parse_layer_key(&KEY_H.to_string()).is_err());
        assert!(parse_layer_key(&KEY_K.to_string()).is_err());
        // The arrow keys the layer emits are rejected too.
        assert!(parse_layer_key(&KEY_LEFT_ARROW.to_string()).is_err());
        assert!(parse_layer_key(&KEY_UP_ARROW.to_string()).is_err());
    }

    #[test]
    fn reports_canonical_key_names() {
        assert_eq!(layer_key_name(KEY_SEMICOLON), Some("semicolon"));
        assert_eq!(layer_key_name(named_key("quote")), Some("quote"));
        assert_eq!(
            layer_key_name(named_key("right_command")),
            Some("right_command")
        );
        assert_eq!(
            layer_key_name(named_key("left_command")),
            Some("left_command")
        );
        assert_eq!(layer_key_name(999), None);
    }

    #[test]
    fn a_modifier_layer_key_drives_the_layer_like_any_other() {
        let right_command = named_key("right_command");
        let mut state = LayerState::new();
        assert_eq!(
            state.on_key(right_command, Down, right_command, NO_FLAGS),
            Suppress
        );
        assert_eq!(
            state.on_key(right_command, Down, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(
            state.on_key(right_command, Up, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(
            state.on_key(right_command, Up, right_command, NO_FLAGS),
            Suppress
        );
        // Used with a non-hjkl key it adds the shortcut modifier.
        assert_eq!(
            state.on_key(right_command, Down, right_command, NO_FLAGS),
            Suppress
        );
        assert_eq!(
            state.on_key(right_command, Down, KEY_A, NO_FLAGS),
            AddShortcutModifier
        );
    }
}
