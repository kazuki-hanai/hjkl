//! A tiny macOS keyboard remapper:
//!
//! - tap `;` by itself -> `;`
//! - hold `;` and press `h/j/k/l` -> left/down/up/right arrow
//! - hold `;` and press another key -> Command + that key
//!
//! This implements the same composed behavior as the original
//! Karabiner-Elements setup without depending on Karabiner at runtime.

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = macos::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("hjkl-for-mac only supports macOS.");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::os::raw::c_long;
    use std::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicPtr, Ordering};

    type Boolean = bool;
    type CFAllocatorRef = *const c_void;
    type CFIndex = c_long;
    type CFMachPortRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFStringRef = *const c_void;
    type CFTypeRef = *const c_void;
    type CGEventRef = *mut c_void;
    type CGEventTapProxy = *mut c_void;
    type CGEventType = u32;
    type CGEventMask = u64;
    type CGEventField = u32;
    type CGEventFlags = u64;
    type CGKeyCode = u16;
    type UniCharCount = u64;

    type CGEventTapCallBack =
        unsafe extern "C" fn(CGEventTapProxy, CGEventType, CGEventRef, *mut c_void) -> CGEventRef;

    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

    const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
    const K_CG_EVENT_KEY_UP: CGEventType = 11;
    const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: CGEventType = 0xFFFF_FFFE;
    const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: CGEventType = 0xFFFF_FFFF;

    const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
    const K_CG_EVENT_SOURCE_USER_DATA: CGEventField = 42;

    const KEY_H: CGKeyCode = 4;
    const KEY_L: CGKeyCode = 37;
    const KEY_J: CGKeyCode = 38;
    const KEY_K: CGKeyCode = 40;
    const KEY_SEMICOLON: CGKeyCode = 41;

    const KEY_LEFT_ARROW: CGKeyCode = 123;
    const KEY_RIGHT_ARROW: CGKeyCode = 124;
    const KEY_DOWN_ARROW: CGKeyCode = 125;
    const KEY_UP_ARROW: CGKeyCode = 126;

    const K_CG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 1 << 20;

    // Marker placed on events synthesized by this process. Without it, the
    // event tap would see its own synthetic semicolon key events and suppress
    // them again.
    const SYNTHETIC_EVENT_TAG: i64 = 0x686A_6B6C_5F72_7374; // "hjkl_rst"

    static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());
    static EVENT_TAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

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

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: CGEventMask,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;

        fn CGEventTapEnable(tap: CFMachPortRef, enable: Boolean);
        fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
        fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
        fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
        fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
        fn CGEventKeyboardSetUnicodeString(
            event: CGEventRef,
            string_length: UniCharCount,
            unicode_string: *const u16,
        );
        fn CGEventCreateKeyboardEvent(
            source: *const c_void,
            virtual_key: CGKeyCode,
            key_down: Boolean,
        ) -> CGEventRef;
        fn CGEventPost(tap: u32, event: CGEventRef);
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopCommonModes: CFStringRef;

        fn CFMachPortCreateRunLoopSource(
            allocator: CFAllocatorRef,
            port: CFMachPortRef,
            order: CFIndex,
        ) -> CFRunLoopSourceRef;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(
            run_loop: CFRunLoopRef,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        fn CFRunLoopRun();
        fn CFRelease(cf: CFTypeRef);
    }

    pub fn run() -> Result<(), String> {
        if std::env::args().any(|arg| arg == "-h" || arg == "--help") {
            print_help();
            return Ok(());
        }

        let mask = event_mask(K_CG_EVENT_KEY_DOWN) | event_mask(K_CG_EVENT_KEY_UP);

        let tap = unsafe {
            CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                mask,
                event_callback,
                ptr::null_mut(),
            )
        };

        if tap.is_null() {
            return Err("Failed to create a keyboard event tap.\n\
                 Grant this terminal/binary permission in macOS System Settings:\n\
                 Privacy & Security -> Accessibility, and if necessary Input Monitoring.\n\
                 Then restart the terminal and run `cargo run --release` again."
                .to_string());
        }

        EVENT_TAP.store(tap, Ordering::SeqCst);

        let source = unsafe { CFMachPortCreateRunLoopSource(ptr::null(), tap, 0) };
        if source.is_null() {
            unsafe {
                CFRelease(tap.cast());
            }
            return Err("Failed to create a run loop source for the event tap.".to_string());
        }

        unsafe {
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
        }

        println!("hjkl-for-mac is running.");
        println!("Tap ';' alone for ';'. Hold ';' + h/j/k/l for left/down/up/right arrows.");
        println!("Hold ';' + another key to send Command + that key.");
        println!("Keep this process running. Press Ctrl-C to stop.");

        unsafe {
            CFRunLoopRun();
            CFRelease(source.cast());
            CFRelease(tap.cast());
        }

        Ok(())
    }

    fn print_help() {
        println!(
            "\
hjkl-for-mac

USAGE:
    cargo run --release
    target/release/hjkl-for-mac

BEHAVIOR:
    ;          -> ;     (when tapped by itself)
    ; + h      -> Left Arrow
    ; + j      -> Down Arrow
    ; + k      -> Up Arrow
    ; + l      -> Right Arrow
    ; + other  -> Command + other

NOTES:
    The program must keep running to remap keys.
    macOS will require Accessibility/Input Monitoring permission for the
    terminal app or for this binary.
"
        );
    }

    unsafe extern "C" fn event_callback(
        _proxy: CGEventTapProxy,
        event_type: CGEventType,
        event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
            || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
        {
            let tap = EVENT_TAP.load(Ordering::SeqCst);
            if !tap.is_null() {
                unsafe {
                    CGEventTapEnable(tap, true);
                }
            }
            return event;
        }

        if event.is_null() {
            return event;
        }

        let user_data = unsafe { CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) };
        if user_data == SYNTHETIC_EVENT_TAG {
            return event;
        }

        if event_type != K_CG_EVENT_KEY_DOWN && event_type != K_CG_EVENT_KEY_UP {
            return event;
        }

        let key_code = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };
        if !(0..=u16::MAX as i64).contains(&key_code) {
            return event;
        }
        let key_code = key_code as CGKeyCode;

        let mut state = match STATE.lock() {
            Ok(state) => state,
            Err(_) => return event,
        };

        if let Some(key_bit) = hjkl_key_bit(key_code) {
            if state.mapped_keys_down & key_bit != 0 {
                if event_type == K_CG_EVENT_KEY_UP {
                    state.mapped_keys_down &= !key_bit;
                }

                let arrow_key = hjkl_to_arrow(key_code).expect("hjkl bit must have arrow mapping");
                drop(state);
                rewrite_as_arrow(event, arrow_key);
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
                    state.semicolon_flags = unsafe { CGEventGetFlags(event) };
                }
                suppress()
            }
            (K_CG_EVENT_KEY_UP, KEY_SEMICOLON) if state.semicolon_down => {
                let should_post_semicolon = !state.used_as_layer;
                let semicolon_flags = state.semicolon_flags;

                state.clear_semicolon();
                drop(state);

                if should_post_semicolon {
                    post_key(KEY_SEMICOLON, semicolon_flags);
                }
                suppress()
            }
            (event_type, key_code) if state.semicolon_down => {
                if let Some(arrow_key) = hjkl_to_arrow(key_code) {
                    if event_type == K_CG_EVENT_KEY_DOWN {
                        state.used_as_layer = true;
                        if let Some(key_bit) = hjkl_key_bit(key_code) {
                            state.mapped_keys_down |= key_bit;
                        }
                        drop(state);

                        rewrite_as_arrow(event, arrow_key);
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
                        add_command_flag(event);
                    }
                    event
                }
            }
            _ => event,
        }
    }

    fn event_mask(event_type: CGEventType) -> CGEventMask {
        1u64 << event_type
    }

    fn suppress() -> CGEventRef {
        ptr::null_mut()
    }

    fn hjkl_to_arrow(key_code: CGKeyCode) -> Option<CGKeyCode> {
        match key_code {
            KEY_H => Some(KEY_LEFT_ARROW),
            KEY_J => Some(KEY_DOWN_ARROW),
            KEY_K => Some(KEY_UP_ARROW),
            KEY_L => Some(KEY_RIGHT_ARROW),
            _ => None,
        }
    }

    fn hjkl_key_bit(key_code: CGKeyCode) -> Option<u8> {
        match key_code {
            KEY_H => Some(1 << 0),
            KEY_J => Some(1 << 1),
            KEY_K => Some(1 << 2),
            KEY_L => Some(1 << 3),
            _ => None,
        }
    }

    fn rewrite_as_arrow(event: CGEventRef, arrow_key: CGKeyCode) {
        unsafe {
            CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, i64::from(arrow_key));
            // The original event still has the text payload for h/j/k/l. Clear
            // it so apps that inspect Unicode see a real non-text arrow key
            // event.
            CGEventKeyboardSetUnicodeString(event, 0, ptr::null());
        }
    }

    fn add_command_flag(event: CGEventRef) {
        unsafe {
            CGEventSetFlags(event, with_command_flag(CGEventGetFlags(event)));
        }
    }

    fn with_command_flag(flags: CGEventFlags) -> CGEventFlags {
        flags | K_CG_EVENT_FLAG_MASK_COMMAND
    }

    fn post_key(key_code: CGKeyCode, flags: CGEventFlags) {
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
            let shift_flag: CGEventFlags = 1 << 17;
            assert_eq!(
                with_command_flag(shift_flag),
                shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND
            );
            assert_eq!(
                with_command_flag(shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND),
                shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND
            );
        }
    }
}
