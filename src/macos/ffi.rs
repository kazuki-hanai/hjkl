//! Raw FFI declarations for CoreFoundation and ApplicationServices.
//!
//! Nothing in this module should contain logic; it only mirrors the C API so
//! the sibling modules can wrap it. `Boolean = bool` matches the current
//! usage; auditing the exact CoreFoundation ABI is a tracked follow-up.

use std::ffi::c_void;
use std::os::raw::c_long;

pub(crate) type Boolean = bool;
pub(crate) type CFAllocatorRef = *const c_void;
pub(crate) type CFIndex = c_long;
pub(crate) type CFDictionaryRef = *const c_void;
pub(crate) type CFBooleanRef = *const c_void;
pub(crate) type CFMachPortRef = *mut c_void;
pub(crate) type CFRunLoopRef = *mut c_void;
pub(crate) type CFRunLoopSourceRef = *mut c_void;
pub(crate) type CFStringRef = *const c_void;
pub(crate) type CFTypeRef = *const c_void;
pub(crate) type CGEventRef = *mut c_void;
pub(crate) type CGEventTapProxy = *mut c_void;
pub(crate) type CGEventType = u32;
pub(crate) type CGEventMask = u64;
pub(crate) type CGEventField = u32;
pub(crate) type CGEventFlags = u64;
pub(crate) type CGKeyCode = u16;
pub(crate) type UniCharCount = u64;

pub(crate) type CGEventTapCallBack =
    unsafe extern "C" fn(CGEventTapProxy, CGEventType, CGEventRef, *mut c_void) -> CGEventRef;

pub(crate) const K_CG_HID_EVENT_TAP: u32 = 0;
pub(crate) const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
pub(crate) const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

pub(crate) const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
pub(crate) const K_CG_EVENT_KEY_UP: CGEventType = 11;
/// Emitted when a modifier key changes state (there is no key-down/up for
/// modifiers). Needed to use a modifier key as the layer key.
pub(crate) const K_CG_EVENT_FLAGS_CHANGED: CGEventType = 12;
pub(crate) const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: CGEventType = 0xFFFF_FFFE;
pub(crate) const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: CGEventType = 0xFFFF_FFFF;

pub(crate) const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
pub(crate) const K_CG_EVENT_SOURCE_USER_DATA: CGEventField = 42;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub(crate) fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    pub(crate) fn CGEventTapEnable(tap: CFMachPortRef, enable: Boolean);
    pub(crate) fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    pub(crate) fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
    pub(crate) fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    pub(crate) fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
    pub(crate) fn CGEventKeyboardSetUnicodeString(
        event: CGEventRef,
        string_length: UniCharCount,
        unicode_string: *const u16,
    );
    pub(crate) fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: CGKeyCode,
        key_down: Boolean,
    ) -> CGEventRef;
    pub(crate) fn CGEventPost(tap: u32, event: CGEventRef);
    pub(crate) fn AXIsProcessTrusted() -> Boolean;
    pub(crate) static kAXTrustedCheckOptionPrompt: CFStringRef;
    pub(crate) fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub(crate) static kCFRunLoopCommonModes: CFStringRef;
    pub(crate) static kCFBooleanTrue: CFBooleanRef;

    pub(crate) fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: CFIndex,
        key_call_backs: *const c_void,
        value_call_backs: *const c_void,
    ) -> CFDictionaryRef;

    pub(crate) fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;
    pub(crate) fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub(crate) fn CFRunLoopAddSource(
        run_loop: CFRunLoopRef,
        source: CFRunLoopSourceRef,
        mode: CFStringRef,
    );
    pub(crate) fn CFRunLoopRun();
    pub(crate) fn CFRelease(cf: CFTypeRef);
}
