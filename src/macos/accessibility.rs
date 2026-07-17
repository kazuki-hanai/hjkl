//! Accessibility (AX) permission checks and prompting.

use std::ffi::c_void;
use std::ptr;

use crate::cli::COMMAND_NAME;
use crate::error::Result;
use crate::macos::ffi::{
    AXIsProcessTrusted, AXIsProcessTrustedWithOptions, CFDictionaryCreate, CFRelease,
    kAXTrustedCheckOptionPrompt, kCFBooleanTrue,
};

pub(crate) fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Ask macOS to show the Accessibility permission prompt for this process
/// (a no-op if permission is already granted). Returns whether the process
/// is currently trusted.
pub(crate) fn request_prompt() -> bool {
    let key = unsafe { kAXTrustedCheckOptionPrompt.cast::<c_void>() };
    let value = unsafe { kCFBooleanTrue.cast::<c_void>() };
    let keys = [key];
    let values = [value];

    let options = unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        )
    };

    if options.is_null() {
        return is_trusted();
    }

    let trusted = unsafe { AXIsProcessTrustedWithOptions(options) };
    unsafe {
        CFRelease(options.cast());
    }
    trusted
}

/// Implementation of `hjkl permissions`.
pub(crate) fn request_permissions() -> Result<()> {
    let trusted = request_prompt();
    if trusted {
        println!("Accessibility permission is already granted.");
    } else {
        println!("Requested Accessibility permission for {COMMAND_NAME}.");
        println!("If macOS opened System Settings, enable {COMMAND_NAME} there.");
        println!("If {COMMAND_NAME} is not listed, add this binary manually:");
        match std::env::current_exe() {
            Ok(path) => println!("  {}", path.display()),
            Err(_) => println!("  ~/.local/bin/{COMMAND_NAME}"),
        }
        println!("Then run `{COMMAND_NAME} restart`.");
    }
    Ok(())
}
