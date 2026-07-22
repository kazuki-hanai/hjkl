//! A tiny keyboard remapper:
//!
//! - tap `;` by itself -> `;`
//! - hold `;` and press `h/j/k/l` -> left/down/up/right arrow
//! - hold `;` and press another key -> the platform shortcut modifier + that key
//!
//! This implements the same composed behavior as the original
//! Karabiner-Elements setup without depending on Karabiner at runtime.

mod app;
mod cli;
mod error;
mod help;
mod keymap;
mod platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
