//! Event tap callback: a thin FFI adapter around the pure semicolon-layer
//! state machine in [`crate::keymap`].

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::keymap::{self, Action, DEFAULT_LAYER_KEY, KeyCode, KeyDirection, LayerState};
use crate::macos::event;
use crate::macos::event_tap;
use crate::macos::ffi::{
    CGEventRef, CGEventTapProxy, CGEventType, CGKeyCode, K_CG_EVENT_FLAGS_CHANGED,
    K_CG_EVENT_KEY_DOWN, K_CG_EVENT_KEY_UP, K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT,
    K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT,
};

static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());

/// The configured layer ("super") key. Set once at daemon startup before the
/// event loop begins, then only read from the callback.
static LAYER_KEY: AtomicU16 = AtomicU16::new(DEFAULT_LAYER_KEY);

/// Configure which key acts as the layer key. Call before `run_event_loop`.
pub(crate) fn set_layer_key(key_code: KeyCode) {
    LAYER_KEY.store(key_code, Ordering::SeqCst);
}

pub(crate) fn layer_key() -> KeyCode {
    LAYER_KEY.load(Ordering::SeqCst)
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

    if event_type != K_CG_EVENT_KEY_DOWN
        && event_type != K_CG_EVENT_KEY_UP
        && event_type != K_CG_EVENT_FLAGS_CHANGED
    {
        return event;
    }

    let key_code = event::key_code(event);
    if !(0..=u16::MAX as i64).contains(&key_code) {
        return event;
    }
    let key_code = key_code as CGKeyCode;

    let layer_key = LAYER_KEY.load(Ordering::SeqCst);

    // Modifier keys have no key-down/up; their state arrives as flagsChanged.
    // Derive a direction from the device flag bit for the configured modifier,
    // and ignore every other flagsChanged event.
    let direction = if event_type == K_CG_EVENT_FLAGS_CHANGED {
        match keymap::modifier_device_flag(layer_key) {
            Some(mask) if key_code == layer_key => {
                if event::flags(event) & mask != 0 {
                    KeyDirection::Down
                } else {
                    KeyDirection::Up
                }
            }
            _ => return event,
        }
    } else if event_type == K_CG_EVENT_KEY_DOWN {
        KeyDirection::Down
    } else {
        KeyDirection::Up
    };

    let action = match STATE.lock() {
        Ok(mut state) => state.on_key(layer_key, direction, key_code, event::flags(event)),
        Err(_) => return event,
    };

    // When the layer key is itself a modifier, that modifier is physically held
    // (we only suppress its flagsChanged, not the hardware state), so its flag
    // rides on the events the layer produces. Strip it so arrows stay bare and
    // "layer + other" becomes exactly the platform shortcut modifier + other.
    let layer_modifier_mask = keymap::modifier_clear_mask(layer_key);

    match action {
        Action::PassThrough => {
            // Keep the layer modifier off every event delivered while it is
            // held, matching the RewriteArrow/AddShortcutModifier arms (no-op
            // when the modifier is not actually held, since its bit is then
            // absent).
            if let Some(mask) = layer_modifier_mask {
                event::clear_flags(event, mask);
            }
            event
        }
        Action::Suppress => event::suppress(),
        Action::RewriteArrow(arrow_key) => {
            event::rewrite_as_arrow(event, arrow_key);
            if let Some(mask) = layer_modifier_mask {
                event::clear_flags(event, mask);
            }
            event
        }
        Action::PostLayerKeyAndSuppress(flags) => {
            // Tapping a modifier layer key alone does nothing (a synthesized
            // modifier press would be a no-op anyway); a normal key is replayed.
            if !keymap::is_modifier(layer_key) {
                event::post_key(layer_key, flags);
            }
            event::suppress()
        }
        Action::AddShortcutModifier => {
            if let Some(mask) = layer_modifier_mask {
                event::clear_flags(event, mask);
            }
            event::add_shortcut_modifier_flag(event);
            event
        }
    }
}
