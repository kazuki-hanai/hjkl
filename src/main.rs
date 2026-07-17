//! A tiny macOS keyboard remapper:
//!
//! - tap `;` by itself -> `;`
//! - hold `;` and press `h/j/k/l` -> left/down/up/right arrow
//! - hold `;` and press another key -> Command + that key
//!
//! This implements the same composed behavior as the original
//! Karabiner-Elements setup without depending on Karabiner at runtime.

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod cli;
#[cfg(target_os = "macos")]
mod error;
#[cfg(target_os = "macos")]
mod help;
#[cfg(target_os = "macos")]
mod keymap;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = app::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("hjkl only supports macOS.");
    std::process::exit(1);
}
