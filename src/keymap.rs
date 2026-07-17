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
    /// Post a synthetic semicolon press with the captured modifier flags,
    /// and swallow the original event.
    PostSemicolonAndSuppress(EventFlags),
    /// Add the Command modifier to the event, then deliver it.
    AddCommandFlag,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerState {
    semicolon_down: bool,
    used_as_layer: bool,
    command_layer_active: bool,
    semicolon_flags: EventFlags,
    mapped_keys_down: u8,
}

impl LayerState {
    pub(crate) const fn new() -> Self {
        Self {
            semicolon_down: false,
            used_as_layer: false,
            command_layer_active: false,
            semicolon_flags: 0,
            mapped_keys_down: 0,
        }
    }

    fn clear_semicolon(&mut self) {
        self.semicolon_down = false;
        self.used_as_layer = false;
        self.command_layer_active = false;
        self.semicolon_flags = 0;
    }

    /// Advance the state machine by one keyboard event and return the action
    /// to perform. `flags` are the event's modifier flags; they are only
    /// consulted when semicolon goes down, so the delayed semicolon can be
    /// replayed with the modifiers that were held at press time.
    pub(crate) fn on_key(
        &mut self,
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
            (KeyDirection::Down, KEY_SEMICOLON) => {
                // Delay semicolon until key-up. If another key is pressed in
                // between, the semicolon became the layer key and should not
                // be emitted as text.
                if !self.semicolon_down {
                    self.semicolon_down = true;
                    self.used_as_layer = false;
                    self.semicolon_flags = flags;
                }
                Action::Suppress
            }
            (KeyDirection::Up, KEY_SEMICOLON) if self.semicolon_down => {
                let should_post_semicolon = !self.used_as_layer;
                let semicolon_flags = self.semicolon_flags;

                self.clear_semicolon();

                if should_post_semicolon {
                    Action::PostSemicolonAndSuppress(semicolon_flags)
                } else {
                    Action::Suppress
                }
            }
            (direction, key_code) if self.semicolon_down => {
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
                    // Karabiner's first rule turns semicolon into
                    // right_command when it is used with any other key. We
                    // emulate that for normal shortcuts by adding the Command
                    // flag to non-hjkl events while the semicolon layer is
                    // held. hjkl is handled above and intentionally becomes a
                    // plain arrow key instead.
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
    fn semicolon_tapped_alone_is_replayed_on_key_up() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(Up, KEY_SEMICOLON, NO_FLAGS),
            PostSemicolonAndSuppress(NO_FLAGS)
        );
    }

    #[test]
    fn semicolon_replay_keeps_modifiers_held_at_press_time() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, SHIFT_FLAG), Suppress);
        // Modifiers at key-up time do not matter; press-time flags win.
        assert_eq!(
            state.on_key(Up, KEY_SEMICOLON, NO_FLAGS),
            PostSemicolonAndSuppress(SHIFT_FLAG)
        );
    }

    #[test]
    fn semicolon_key_repeat_stays_suppressed_without_resetting_capture() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, SHIFT_FLAG), Suppress);
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(Up, KEY_SEMICOLON, NO_FLAGS),
            PostSemicolonAndSuppress(SHIFT_FLAG)
        );
    }

    #[test]
    fn holding_semicolon_turns_hjkl_into_arrows_and_swallows_semicolon() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(Down, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        assert_eq!(
            state.on_key(Up, KEY_H, NO_FLAGS),
            RewriteArrow(KEY_LEFT_ARROW)
        );
        // Semicolon was used as a layer key, so no semicolon is emitted.
        assert_eq!(state.on_key(Up, KEY_SEMICOLON, NO_FLAGS), Suppress);
    }

    #[test]
    fn arrow_key_up_is_still_rewritten_after_semicolon_release() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(Down, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        // Semicolon released while j is still held.
        assert_eq!(state.on_key(Up, KEY_SEMICOLON, NO_FLAGS), Suppress);
        // The j key-up must still become a down-arrow key-up, otherwise the
        // arrow key would be stuck down.
        assert_eq!(
            state.on_key(Up, KEY_J, NO_FLAGS),
            RewriteArrow(KEY_DOWN_ARROW)
        );
        // With the layer gone, j is a plain key again.
        assert_eq!(state.on_key(Down, KEY_J, NO_FLAGS), PassThrough);
    }

    #[test]
    fn key_repeat_of_mapped_key_keeps_rewriting_while_held() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(
            state.on_key(Down, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
        // Auto-repeat key-downs while held keep being rewritten.
        assert_eq!(
            state.on_key(Down, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
        assert_eq!(
            state.on_key(Up, KEY_K, NO_FLAGS),
            RewriteArrow(KEY_UP_ARROW)
        );
    }

    #[test]
    fn holding_semicolon_adds_command_to_other_keys() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        assert_eq!(state.on_key(Down, KEY_A, NO_FLAGS), AddCommandFlag);
        assert_eq!(state.on_key(Up, KEY_A, NO_FLAGS), AddCommandFlag);
        // Semicolon acted as a modifier, so it is not emitted.
        assert_eq!(state.on_key(Up, KEY_SEMICOLON, NO_FLAGS), Suppress);
        // After release the layer is fully reset.
        assert_eq!(state.on_key(Down, KEY_A, NO_FLAGS), PassThrough);
    }

    #[test]
    fn hjkl_key_up_without_prior_layer_press_passes_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_SEMICOLON, NO_FLAGS), Suppress);
        // h was pressed before the layer was activated, so its key-up is not
        // a mapped key and must pass through unchanged.
        assert_eq!(state.on_key(Up, KEY_H, NO_FLAGS), PassThrough);
    }

    #[test]
    fn semicolon_key_up_without_key_down_passes_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Up, KEY_SEMICOLON, NO_FLAGS), PassThrough);
    }

    #[test]
    fn keys_without_layer_pass_through() {
        let mut state = LayerState::new();
        assert_eq!(state.on_key(Down, KEY_A, NO_FLAGS), PassThrough);
        assert_eq!(state.on_key(Down, KEY_H, NO_FLAGS), PassThrough);
        assert_eq!(state.on_key(Up, KEY_H, NO_FLAGS), PassThrough);
    }
}
