//! Event tap callback: a thin FFI adapter around the pure semicolon-layer
//! state machine in [`crate::keymap`].

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::keymap::{Action, DEFAULT_LAYER_KEY, KeyCode, KeyDirection, LayerState};
use crate::macos::event;
use crate::macos::event_tap;
use crate::macos::ffi::{
    CGEventRef, CGEventTapProxy, CGEventType, CGKeyCode, K_CG_EVENT_KEY_DOWN, K_CG_EVENT_KEY_UP,
    K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT, K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT,
};

static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());

/// The configured layer ("super") key. Set once at daemon startup before the
/// event loop begins, then only read from the callback.
static LAYER_KEY: AtomicU16 = AtomicU16::new(DEFAULT_LAYER_KEY);

/// Configure which key acts as the layer key. Call before `run_event_loop`.
pub(crate) fn set_layer_key(key_code: KeyCode) {
    LAYER_KEY.store(key_code, Ordering::SeqCst);
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

    let direction = match event_type {
        K_CG_EVENT_KEY_DOWN => KeyDirection::Down,
        K_CG_EVENT_KEY_UP => KeyDirection::Up,
        _ => return event,
    };

    let key_code = event::key_code(event);
    if !(0..=u16::MAX as i64).contains(&key_code) {
        return event;
    }
    let key_code = key_code as CGKeyCode;

    let layer_key = LAYER_KEY.load(Ordering::SeqCst);

    let action = match STATE.lock() {
        Ok(mut state) => state.on_key(layer_key, direction, key_code, event::flags(event)),
        Err(_) => return event,
    };

    match action {
        Action::PassThrough => event,
        Action::Suppress => event::suppress(),
        Action::RewriteArrow(arrow_key) => {
            event::rewrite_as_arrow(event, arrow_key);
            event
        }
        Action::PostLayerKeyAndSuppress(flags) => {
            event::post_key(layer_key, flags);
            event::suppress()
        }
        Action::AddCommandFlag => {
            event::add_command_flag(event);
            event
        }
    }
}
