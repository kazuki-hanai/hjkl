//! Pure semicolon-layer mapping logic and state machine.
//!
//! Everything here is plain data and arithmetic — no FFI — so the full layer
//! behavior can be tested without macOS Accessibility permissions. The event
//! tap callback in `macos::remapper` translates each keyboard event into an
//! [`LayerState::on_key`] call and performs the returned [`Action`].

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

/// The layer (a.k.a. "super") key used when none is configured.
pub(crate) const DEFAULT_LAYER_KEY: KeyCode = KEY_SEMICOLON;

/// `kCGEventFlagMaskCommand`.
pub(crate) const COMMAND_FLAG_MASK: EventFlags = 1 << 20;

/// Friendly names accepted for the layer key, paired with their macOS virtual
/// key codes. Both plain keys and modifier keys are listed. Names are matched
/// case- and separator-insensitively (see [`normalize_key_name`]); the first
/// entry for a given code is the canonical one used for display.
const LAYER_KEY_NAMES: &[(&str, KeyCode)] = &[
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

/// Modifier key codes usable as the layer key, paired with the
/// device-dependent CGEvent flag bit that is set while that specific physical
/// key is held. The device bit distinguishes left from right (unlike the
/// general modifier masks) and lets us tell a `flagsChanged` down from an up.
const MODIFIER_DEVICE_FLAGS: &[(KeyCode, EventFlags)] = &[
    (55, 0x0000_0008), // left command  (NX_DEVICELCMDKEYMASK)
    (54, 0x0000_0010), // right command (NX_DEVICERCMDKEYMASK)
    (56, 0x0000_0002), // left shift     (NX_DEVICELSHIFTKEYMASK)
    (60, 0x0000_0004), // right shift    (NX_DEVICERSHIFTKEYMASK)
    (59, 0x0000_0001), // left control   (NX_DEVICELCTLKEYMASK)
    (62, 0x0000_2000), // right control  (NX_DEVICERCTLKEYMASK)
    (58, 0x0000_0020), // left option    (NX_DEVICELALTKEYMASK)
    (61, 0x0000_0040), // right option   (NX_DEVICERALTKEYMASK)
];

/// Caps Lock (57) and Fn (63) are modifiers with special/toggle semantics that
/// this tool does not support as a layer key.
const UNSUPPORTED_MODIFIER_CODES: &[KeyCode] = &[57, 63];

fn normalize_key_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '-' && *c != ' ')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether a key code is a modifier the layer machine drives via
/// `flagsChanged` rather than key-down/up events.
pub(crate) fn is_modifier(key_code: KeyCode) -> bool {
    MODIFIER_DEVICE_FLAGS
        .iter()
        .any(|(code, _)| *code == key_code)
}

/// The device-dependent flag bit set while the given modifier key is held, if
/// it is a supported modifier.
pub(crate) fn modifier_device_flag(key_code: KeyCode) -> Option<EventFlags> {
    MODIFIER_DEVICE_FLAGS
        .iter()
        .find(|(code, _)| *code == key_code)
        .map(|(_, mask)| *mask)
}

/// All flag bits (general mask + device bit) contributed by holding the given
/// modifier key. When the modifier is the layer key, these are stripped from
/// the events the layer emits so, e.g., `right_command + j` produces a bare
/// Down arrow rather than Command+Down.
///
/// Limitation: the general mask (e.g. `COMMAND_FLAG_MASK`) is shared by the
/// left and right key of the same modifier. If the user genuinely holds the
/// opposite-side key of the same modifier while the layer is active, clearing
/// the general bit also drops that key's contribution. This is a rare edge and
/// accepted; the device bit still distinguishes the two physically.
pub(crate) fn modifier_clear_mask(key_code: KeyCode) -> Option<EventFlags> {
    let device = modifier_device_flag(key_code)?;
    let general: EventFlags = match key_code {
        55 | 54 => COMMAND_FLAG_MASK, // command  (1 << 20)
        56 | 60 => 1 << 17,           // shift
        58 | 61 => 1 << 19,           // option / alt
        59 | 62 => 1 << 18,           // control
        _ => 0,
    };
    Some(device | general)
}

/// Parse a user-supplied layer-key spec — either a friendly name
/// (`semicolon`, `quote`, `right_command` …, case- and separator-insensitive)
/// or a raw decimal macOS virtual key code — into a validated key code.
pub(crate) fn parse_layer_key(spec: &str) -> Result<KeyCode, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("layer key is empty".to_string());
    }

    let normalized = normalize_key_name(trimmed);

    let key_code = if let Some((_, code)) = LAYER_KEY_NAMES
        .iter()
        .find(|(name, _)| normalize_key_name(name) == normalized)
    {
        *code
    } else if let Ok(code) = normalized.parse::<KeyCode>() {
        code
    } else {
        return Err(format!(
            "unknown layer key '{spec}'. Use a name like 'semicolon', 'quote', or \
             'right_command', or a numeric macOS key code."
        ));
    };

    validate_layer_key(key_code)?;
    Ok(key_code)
}

fn validate_layer_key(key_code: KeyCode) -> Result<(), String> {
    if UNSUPPORTED_MODIFIER_CODES.contains(&key_code) {
        return Err(format!(
            "key code {key_code} (Caps Lock/Fn) is not supported as the layer key."
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
    LAYER_KEY_NAMES
        .iter()
        .find(|(_, code)| *code == key_code)
        .map(|(name, _)| *name)
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

pub(crate) fn with_command_flag(flags: EventFlags) -> EventFlags {
    flags | COMMAND_FLAG_MASK
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
    /// Add the Command modifier to the event, then deliver it.
    AddCommandFlag,
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
                    // Karabiner's first rule turns the layer key into
                    // right_command when it is used with any other key. We
                    // emulate that for normal shortcuts by adding the Command
                    // flag to non-hjkl events while the layer is held. hjkl is
                    // handled above and intentionally becomes a plain arrow key
                    // instead.
                    if direction == KeyDirection::Down {
                        self.used_as_layer = true;
                        self.command_layer_active = true;
                    }
                    if self.command_layer_active {
                        Action::AddCommandFlag
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
    fn command_flag_is_added_without_dropping_other_flags() {
        assert_eq!(
            with_command_flag(SHIFT_FLAG),
            SHIFT_FLAG | COMMAND_FLAG_MASK
        );
        assert_eq!(
            with_command_flag(SHIFT_FLAG | COMMAND_FLAG_MASK),
            SHIFT_FLAG | COMMAND_FLAG_MASK
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
    fn holding_layer_adds_command_to_other_keys() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(LAYER, Down, LAYER, NO_FLAGS), Suppress);
        assert_eq!(state.on_key(LAYER, Down, KEY_A, NO_FLAGS), AddCommandFlag);
        assert_eq!(state.on_key(LAYER, Up, KEY_A, NO_FLAGS), AddCommandFlag);
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
        // Configure quote (39) as the layer key.
        const QUOTE: KeyCode = 39;
        let mut state = LayerState::new();

        // Semicolon is now just a normal key and passes through.
        assert_eq!(
            state.on_key(QUOTE, Down, KEY_SEMICOLON, NO_FLAGS),
            PassThrough
        );

        // Quote drives the layer: hold quote + j -> down arrow.
        assert_eq!(state.on_key(QUOTE, Down, QUOTE, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(QUOTE, Down, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        assert_eq!(
            state.on_key(QUOTE, Up, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        assert_eq!(state.on_key(QUOTE, Up, QUOTE, NO_FLAGS), Suppress);

        // Tapping quote alone replays quote.
        assert_eq!(state.on_key(QUOTE, Down, QUOTE, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(QUOTE, Up, QUOTE, NO_FLAGS),
            PostLayerKeyAndSuppress(NO_FLAGS)
        );
    }

    #[test]
    fn parses_layer_key_names_and_codes() {
        assert_eq!(parse_layer_key("semicolon"), Ok(41));
        assert_eq!(parse_layer_key("quote"), Ok(39));
        assert_eq!(parse_layer_key("apostrophe"), Ok(39));
        // Case- and separator-insensitive.
        assert_eq!(parse_layer_key("Right-Bracket"), Ok(30));
        assert_eq!(parse_layer_key("  TAB  "), Ok(48));
        // Raw numeric codes.
        assert_eq!(parse_layer_key("39"), Ok(39));
        assert_eq!(parse_layer_key("50"), Ok(50));
    }

    #[test]
    fn parses_modifier_layer_keys_with_left_right() {
        assert_eq!(parse_layer_key("right_command"), Ok(54));
        assert_eq!(parse_layer_key("left_command"), Ok(55));
        assert_eq!(parse_layer_key("Right Option"), Ok(61));
        assert_eq!(parse_layer_key("left-ctrl"), Ok(59));
        assert_eq!(parse_layer_key("right_shift"), Ok(60));
        // Numeric modifier codes are accepted too.
        assert_eq!(parse_layer_key("54"), Ok(54));

        assert!(is_modifier(54));
        assert!(is_modifier(55));
        assert!(!is_modifier(41)); // semicolon is not a modifier
        // Left and right of the same modifier get distinct device flag bits.
        assert_eq!(modifier_device_flag(55), Some(0x8)); // left command
        assert_eq!(modifier_device_flag(54), Some(0x10)); // right command
        assert_ne!(modifier_device_flag(55), modifier_device_flag(54));
        assert_eq!(modifier_device_flag(41), None);
    }

    #[test]
    fn modifier_clear_mask_includes_device_and_general_bits() {
        // right command: device bit 0x10 + general command mask (1 << 20).
        assert_eq!(modifier_clear_mask(54), Some(0x10 | COMMAND_FLAG_MASK));
        // left shift: device bit 0x2 + general shift mask (1 << 17).
        assert_eq!(modifier_clear_mask(56), Some(0x2 | (1 << 17)));
        // Non-modifier keys contribute no mask.
        assert_eq!(modifier_clear_mask(41), None);
    }

    #[test]
    fn rejects_invalid_layer_keys() {
        assert!(parse_layer_key("").is_err());
        assert!(parse_layer_key("not_a_key").is_err());
        // Caps Lock and Fn are unsupported modifiers.
        assert!(parse_layer_key("57").is_err()); // caps lock
        assert!(parse_layer_key("63").is_err()); // fn
        // h/j/k/l are arrow targets and cannot also be the layer key.
        assert!(parse_layer_key("4").is_err()); // h
        assert!(parse_layer_key("40").is_err()); // k
        // The arrow keys the layer emits are rejected too.
        assert!(parse_layer_key("123").is_err()); // left arrow
        assert!(parse_layer_key("126").is_err()); // up arrow
    }

    #[test]
    fn reports_canonical_key_names() {
        assert_eq!(layer_key_name(41), Some("semicolon"));
        assert_eq!(layer_key_name(39), Some("quote"));
        assert_eq!(layer_key_name(54), Some("right_command"));
        assert_eq!(layer_key_name(55), Some("left_command"));
        assert_eq!(layer_key_name(999), None);
    }

    #[test]
    fn a_modifier_layer_key_drives_the_layer_like_any_other() {
        // right_command as the layer key. The state machine itself is agnostic
        // to modifier-ness; the caller derives direction from flagsChanged.
        const RCMD: KeyCode = 54;
        let mut state = LayerState::new();
        assert_eq!(state.on_key(RCMD, Down, RCMD, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(RCMD, Down, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(
            state.on_key(RCMD, Up, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(state.on_key(RCMD, Up, RCMD, NO_FLAGS), Suppress);
        // Used with a non-hjkl key it adds Command.
        assert_eq!(state.on_key(RCMD, Down, RCMD, NO_FLAGS), Suppress);
        assert_eq!(state.on_key(RCMD, Down, KEY_A, NO_FLAGS), AddCommandFlag);
    }
}
