//! Low-level Windows keyboard hook adapter around the shared layer state.

use std::mem;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, AtomicU16, Ordering};

use crate::cli::COMMAND_NAME;
use crate::error::{Error, Result};
use crate::keymap::{self, Action, DEFAULT_LAYER_KEY, KeyCode, KeyDirection, LayerState};
use crate::windows::{input, service};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static STATE: Mutex<HookState> = Mutex::new(HookState::new());
static LAYER_KEY: AtomicU16 = AtomicU16::new(DEFAULT_LAYER_KEY);
static HOOK: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

struct HookState {
    layer: LayerState,
    shortcut_keys_down: [bool; 256],
    shortcut_key_count: u16,
    shortcut_modifier_down: bool,
}

impl HookState {
    const fn new() -> Self {
        Self {
            layer: LayerState::new(),
            shortcut_keys_down: [false; 256],
            shortcut_key_count: 0,
            shortcut_modifier_down: false,
        }
    }

    fn is_shortcut_key_down(&self, key_code: KeyCode) -> bool {
        self.shortcut_keys_down
            .get(usize::from(key_code))
            .copied()
            .unwrap_or(false)
    }

    fn send_shortcut_key(&mut self, direction: KeyDirection, key_code: KeyCode) {
        match direction {
            KeyDirection::Down => {
                if !self.shortcut_modifier_down {
                    input::press_shortcut_modifier();
                    self.shortcut_modifier_down = true;
                }
                if let Some(is_down) = self.shortcut_keys_down.get_mut(usize::from(key_code))
                    && !*is_down
                {
                    *is_down = true;
                    self.shortcut_key_count = self.shortcut_key_count.saturating_add(1);
                }
                input::send_key_event(key_code, KeyDirection::Down);
            }
            KeyDirection::Up => {
                input::send_key_event(key_code, KeyDirection::Up);
                if let Some(is_down) = self.shortcut_keys_down.get_mut(usize::from(key_code))
                    && *is_down
                {
                    *is_down = false;
                    self.shortcut_key_count = self.shortcut_key_count.saturating_sub(1);
                }
                if self.shortcut_key_count == 0 && self.shortcut_modifier_down {
                    input::release_shortcut_modifier();
                    self.shortcut_modifier_down = false;
                }
            }
        }
    }

    fn release_shortcut_modifier_if_down(&mut self) {
        if self.shortcut_modifier_down {
            input::release_shortcut_modifier();
            self.shortcut_modifier_down = false;
        }
        self.shortcut_keys_down.fill(false);
        self.shortcut_key_count = 0;
    }
}

/// Configure which key acts as the layer key. Call before `run_event_loop`.
pub(crate) fn set_layer_key(key_code: KeyCode) {
    LAYER_KEY.store(key_code, Ordering::SeqCst);
}

pub(crate) fn run_event_loop(service_mode: bool) -> Result<()> {
    if service_mode {
        println!(
            "Running in Windows service foreground mode. Use `{COMMAND_NAME} start` or `{COMMAND_NAME} enable` to run in the background."
        );
    }

    let hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), ptr::null_mut(), 0) };
    if hook.is_null() {
        service::write_health(service::Health::HookFailed);
        return Err(Error::from(
            "Failed to install the Windows low-level keyboard hook.",
        ));
    }

    HOOK.store(hook.cast(), Ordering::SeqCst);
    let _guard = HookGuard(hook);
    service::write_health(service::Health::Ok);

    let layer_key = keymap::layer_key_label(LAYER_KEY.load(Ordering::SeqCst));
    let shortcut_modifier = keymap::shortcut_modifier_name();

    println!("{COMMAND_NAME} is running.");
    println!("Tap {layer_key} alone to emit it.");
    println!("Hold {layer_key} + h/j/k/l for left/down/up/right arrows.");
    println!("Hold {layer_key} + another key to send {shortcut_modifier} + that key.");
    println!("Keep this process running. Press Ctrl-C to stop.");

    let mut message = unsafe { mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result == -1 {
            return Err(Error::from("Failed while waiting for Windows messages."));
        }
        if result == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    if let Ok(mut state) = STATE.lock() {
        state.release_shortcut_modifier_if_down();
    }

    Ok(())
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != HC_ACTION as i32 {
        return call_next(code, wparam, lparam);
    }

    let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    if event.dwExtraInfo == input::SYNTHETIC_EVENT_TAG {
        return call_next(code, wparam, lparam);
    }

    let Some(direction) = message_direction(wparam as u32) else {
        return call_next(code, wparam, lparam);
    };

    if event.vkCode > u32::from(u16::MAX) {
        return call_next(code, wparam, lparam);
    }
    let key_code = event.vkCode as KeyCode;
    let layer_key = LAYER_KEY.load(Ordering::SeqCst);

    let suppress = match STATE.lock() {
        Ok(mut state) => handle_event(&mut state, layer_key, direction, key_code),
        Err(_) => false,
    };

    if suppress {
        1
    } else {
        call_next(code, wparam, lparam)
    }
}

fn handle_event(
    state: &mut HookState,
    layer_key: KeyCode,
    direction: KeyDirection,
    key_code: KeyCode,
) -> bool {
    let action = state.layer.on_key(layer_key, direction, key_code, 0);
    match action {
        Action::PassThrough => {
            if state.is_shortcut_key_down(key_code) {
                state.send_shortcut_key(direction, key_code);
                true
            } else {
                false
            }
        }
        Action::Suppress => true,
        Action::RewriteArrow(arrow_key) => {
            input::send_key_event(arrow_key, direction);
            true
        }
        Action::PostLayerKeyAndSuppress(_) => {
            if !keymap::is_modifier(layer_key) {
                input::post_key(layer_key);
            }
            true
        }
        Action::AddShortcutModifier => {
            state.send_shortcut_key(direction, key_code);
            true
        }
    }
}

fn message_direction(message: u32) -> Option<KeyDirection> {
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyDirection::Down),
        WM_KEYUP | WM_SYSKEYUP => Some(KeyDirection::Up),
        _ => None,
    }
}

fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hook = HOOK.load(Ordering::SeqCst) as HHOOK;
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}

struct HookGuard(HHOOK);

impl Drop for HookGuard {
    fn drop(&mut self) {
        HOOK.store(ptr::null_mut(), Ordering::SeqCst);
        unsafe {
            UnhookWindowsHookEx(self.0);
        }
    }
}
