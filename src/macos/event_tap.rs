//! CGEvent tap creation and run-loop integration.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;
use std::time::Duration;

use crate::cli::COMMAND_NAME;
use crate::error::{Error, Result};
use crate::macos::event;
use crate::macos::ffi::{
    CFMachPortCreateRunLoopSource, CFMachPortRef, CFRelease, CFRunLoopAddSource,
    CFRunLoopGetCurrent, CFRunLoopRun, CGEventMask, CGEventTapCreate, CGEventTapEnable,
    K_CG_EVENT_FLAGS_CHANGED, K_CG_EVENT_KEY_DOWN, K_CG_EVENT_KEY_UP,
    K_CG_EVENT_TAP_OPTION_DEFAULT, K_CG_HEAD_INSERT_EVENT_TAP, K_CG_HID_EVENT_TAP,
    kCFRunLoopCommonModes,
};
use crate::macos::remapper;
use crate::macos::service;

const EVENT_TAP_RETRY_INTERVAL: Duration = Duration::from_secs(30);

static EVENT_TAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

pub(crate) fn run_event_loop(service_mode: bool) -> Result<()> {
    if service_mode {
        println!(
            "Running in launchd foreground mode. Use `{COMMAND_NAME} start` or `{COMMAND_NAME} enable` to run in the background."
        );
    }

    // flagsChanged is tapped so a modifier key can serve as the layer key; for
    // a non-modifier layer key those events simply pass through.
    let mask = event::event_mask(K_CG_EVENT_KEY_DOWN)
        | event::event_mask(K_CG_EVENT_KEY_UP)
        | event::event_mask(K_CG_EVENT_FLAGS_CHANGED);
    let tap = create_event_tap(mask, service_mode)?;

    EVENT_TAP.store(tap, Ordering::SeqCst);

    let source = unsafe { CFMachPortCreateRunLoopSource(ptr::null(), tap, 0) };
    if source.is_null() {
        // Drop the dangling reference before freeing the port so reenable()
        // can never call CGEventTapEnable on freed memory.
        EVENT_TAP.store(ptr::null_mut(), Ordering::SeqCst);
        unsafe {
            CFRelease(tap.cast());
        }
        return Err(Error::from(
            "Failed to create a run loop source for the event tap.",
        ));
    }

    unsafe {
        CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
    }

    // Record that this process actually acquired the keyboard tap so that
    // `start`/`enable`/`restart` and `status` can report the real state.
    service::write_health(service::Health::Ok);

    println!("{COMMAND_NAME} is running.");
    println!("Tap ';' alone for ';'. Hold ';' + h/j/k/l for left/down/up/right arrows.");
    println!("Hold ';' + another key to send Command + that key.");
    println!("Keep this process running. Press Ctrl-C to stop.");

    unsafe {
        CFRunLoopRun();
        // The run loop has exited, so the callback can no longer fire. Clear
        // the shared pointer before freeing the port so a late reenable()
        // cannot dereference freed memory.
        EVENT_TAP.store(ptr::null_mut(), Ordering::SeqCst);
        CFRelease(source.cast());
        CFRelease(tap.cast());
    }

    Ok(())
}

/// Re-enable the tap after macOS disabled it (timeout or user input).
/// Called from the tap callback.
pub(crate) fn reenable() {
    let tap = EVENT_TAP.load(Ordering::SeqCst);
    if !tap.is_null() {
        unsafe {
            CGEventTapEnable(tap, true);
        }
    }
}

fn create_event_tap(mask: CGEventMask, retry_until_available: bool) -> Result<CFMachPortRef> {
    loop {
        let tap = unsafe {
            CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                mask,
                remapper::event_callback,
                ptr::null_mut(),
            )
        };

        if !tap.is_null() {
            return Ok(tap);
        }

        // The tap could not be created, almost always because macOS has not
        // granted this binary permission. Record it so the management
        // commands can surface a real error instead of a false success.
        service::write_health(service::Health::TapFailed);

        if !retry_until_available {
            return Err(Error::from(event_tap_permission_error()));
        }

        eprintln!(
            "{}\nRetrying in {} seconds for launchd foreground mode...",
            event_tap_permission_error(),
            EVENT_TAP_RETRY_INTERVAL.as_secs()
        );
        thread::sleep(EVENT_TAP_RETRY_INTERVAL);
    }
}

fn event_tap_permission_error() -> String {
    "Failed to create a keyboard event tap.\n\
     Grant this terminal/binary permission in macOS System Settings:\n\
     Privacy & Security -> Accessibility, and if necessary Input Monitoring.\n\
     Then restart the terminal or daemon."
        .to_string()
}
