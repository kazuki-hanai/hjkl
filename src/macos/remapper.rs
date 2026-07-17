//! Event tap callback: a thin FFI adapter around the pure semicolon-layer
//! state machine in [`crate::keymap`].

use std::ffi::c_void;
use std::sync::Mutex;

use crate::keymap::{Action, KEY_SEMICOLON, KeyDirection, LayerState};
use crate::macos::event;
use crate::macos::event_tap;
use crate::macos::ffi::{
    CGEventRef, CGEventTapProxy, CGEventType, CGKeyCode, K_CG_EVENT_KEY_DOWN, K_CG_EVENT_KEY_UP,
    K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT, K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT,
};

static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());

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

    let action = match STATE.lock() {
        Ok(mut state) => state.on_key(direction, key_code, event::flags(event)),
        Err(_) => return event,
    };

    match action {
        Action::PassThrough => event,
        Action::Suppress => event::suppress(),
        Action::RewriteArrow(arrow_key) => {
            event::rewrite_as_arrow(event, arrow_key);
            event
        }
        Action::PostSemicolonAndSuppress(flags) => {
            event::post_key(KEY_SEMICOLON, flags);
            event::suppress()
        }
        Action::AddCommandFlag => {
            event::add_command_flag(event);
            event
        }
    }
}
