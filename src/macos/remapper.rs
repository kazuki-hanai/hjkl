//! The semicolon-layer state machine, driven by the event tap callback.

use std::ffi::c_void;
use std::sync::Mutex;

use crate::keymap::{self, KEY_SEMICOLON};
use crate::macos::event;
use crate::macos::event_tap;
use crate::macos::ffi::{
    CGEventFlags, CGEventRef, CGEventTapProxy, CGEventType, CGKeyCode, K_CG_EVENT_KEY_DOWN,
    K_CG_EVENT_KEY_UP, K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT, K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT,
};

static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());

#[derive(Debug, Clone, Copy)]
struct LayerState {
    semicolon_down: bool,
    used_as_layer: bool,
    command_layer_active: bool,
    semicolon_flags: CGEventFlags,
    mapped_keys_down: u8,
}

impl LayerState {
    const fn new() -> Self {
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
}

pub(crate) unsafe extern "C" fn event_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    {
        event_tap::reenable();
        return event;
    }

    if event.is_null() {
        return event;
    }

    if event::user_data(event) == event::SYNTHETIC_EVENT_TAG {
        return event;
    }

    if event_type != K_CG_EVENT_KEY_DOWN && event_type != K_CG_EVENT_KEY_UP {
        return event;
    }

    let key_code = event::key_code(event);
    if !(0..=u16::MAX as i64).contains(&key_code) {
        return event;
    }
    let key_code = key_code as CGKeyCode;

    let mut state = match STATE.lock() {
        Ok(state) => state,
        Err(_) => return event,
    };

    if let Some(key_bit) = keymap::hjkl_key_bit(key_code)
        && state.mapped_keys_down & key_bit != 0
    {
        if event_type == K_CG_EVENT_KEY_UP {
            state.mapped_keys_down &= !key_bit;
        }

        if let Some(arrow_key) = keymap::hjkl_to_arrow(key_code) {
            drop(state);
            event::rewrite_as_arrow(event, arrow_key);
            return event;
        }
    }

    match (event_type, key_code) {
        (K_CG_EVENT_KEY_DOWN, KEY_SEMICOLON) => {
            // Delay semicolon until key-up. If another key is pressed in
            // between, the semicolon became the layer key and should not
            // be emitted as text.
            if !state.semicolon_down {
                state.semicolon_down = true;
                state.used_as_layer = false;
                state.semicolon_flags = event::flags(event);
            }
            event::suppress()
        }
        (K_CG_EVENT_KEY_UP, KEY_SEMICOLON) if state.semicolon_down => {
            let should_post_semicolon = !state.used_as_layer;
            let semicolon_flags = state.semicolon_flags;

            state.clear_semicolon();
            drop(state);

            if should_post_semicolon {
                event::post_key(KEY_SEMICOLON, semicolon_flags);
            }
            event::suppress()
        }
        (event_type, key_code) if state.semicolon_down => {
            if let Some(arrow_key) = keymap::hjkl_to_arrow(key_code) {
                if event_type == K_CG_EVENT_KEY_DOWN {
                    state.used_as_layer = true;
                    if let Some(key_bit) = keymap::hjkl_key_bit(key_code) {
                        state.mapped_keys_down |= key_bit;
                    }
                    drop(state);

                    event::rewrite_as_arrow(event, arrow_key);
                }
                event
            } else {
                // Karabiner's first rule turns semicolon into
                // right_command when it is used with any other key. We
                // emulate that for normal shortcuts by adding the Command
                // flag to non-hjkl events while the semicolon layer is
                // held. hjkl is handled above and intentionally becomes a
                // plain arrow key instead.
                if event_type == K_CG_EVENT_KEY_DOWN {
                    state.used_as_layer = true;
                    state.command_layer_active = true;
                }
                let should_add_command = state.command_layer_active;
                drop(state);

                if should_add_command {
                    event::add_command_flag(event);
                }
                event
            }
        }
        _ => event,
    }
}
