//! Small facade over the active operating-system backend.
//!
//! The rest of the crate should talk to this module rather than importing
//! `macos` or `windows` directly. That keeps the CLI and the layer state
//! machine shared while the event loop, service manager, permissions, and key
//! code tables remain platform-specific.

use crate::error::Result;
use crate::keymap::KeyCode;

#[cfg(target_os = "macos")]
pub(crate) use crate::macos::keys;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) use crate::unsupported::keys;
#[cfg(target_os = "windows")]
pub(crate) use crate::windows::keys;

#[cfg(target_os = "macos")]
pub(crate) const SERVICE_MANAGER: &str = "per-user launchd LaunchAgent";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) const SERVICE_MANAGER: &str = "background service";
#[cfg(target_os = "windows")]
pub(crate) const SERVICE_MANAGER: &str = "per-user Windows scheduled task";

#[cfg(target_os = "macos")]
pub(crate) const PERMISSIONS_NOTE: &str = "macOS will require Accessibility permission for this binary. After granting it, run `hjkl restart`.";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) const PERMISSIONS_NOTE: &str = "This operating system is not supported.";
#[cfg(target_os = "windows")]
pub(crate) const PERMISSIONS_NOTE: &str = "Windows does not require Accessibility permission. Run hjkl as administrator only if you need to remap elevated applications.";

pub(crate) fn set_layer_key(key_code: KeyCode) {
    backend::set_layer_key(key_code);
}

pub(crate) fn run_event_loop(service_mode: bool) -> Result<()> {
    backend::run_event_loop(service_mode)
}

pub(crate) fn start(layer_key: Option<KeyCode>) -> Result<()> {
    backend::start(layer_key)
}

pub(crate) fn stop() -> Result<()> {
    backend::stop()
}

pub(crate) fn restart(layer_key: Option<KeyCode>) -> Result<()> {
    backend::restart(layer_key)
}

pub(crate) fn enable(layer_key: Option<KeyCode>) -> Result<()> {
    backend::enable(layer_key)
}

pub(crate) fn disable() -> Result<()> {
    backend::disable()
}

pub(crate) fn status() -> Result<()> {
    backend::status()
}

pub(crate) fn request_permissions() -> Result<()> {
    backend::request_permissions()
}

#[cfg(target_os = "macos")]
mod backend {
    use crate::error::Result;
    use crate::keymap::KeyCode;
    use crate::macos::{accessibility, event_tap, remapper, service};

    pub(super) fn set_layer_key(key_code: KeyCode) {
        remapper::set_layer_key(key_code);
    }

    pub(super) fn run_event_loop(service_mode: bool) -> Result<()> {
        event_tap::run_event_loop(service_mode)
    }

    pub(super) fn start(layer_key: Option<KeyCode>) -> Result<()> {
        service::start(layer_key)
    }

    pub(super) fn stop() -> Result<()> {
        service::stop()
    }

    pub(super) fn restart(layer_key: Option<KeyCode>) -> Result<()> {
        service::restart(layer_key)
    }

    pub(super) fn enable(layer_key: Option<KeyCode>) -> Result<()> {
        service::enable(layer_key)
    }

    pub(super) fn disable() -> Result<()> {
        service::disable()
    }

    pub(super) fn status() -> Result<()> {
        service::status()
    }

    pub(super) fn request_permissions() -> Result<()> {
        accessibility::request_permissions()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod backend {
    use crate::error::Result;
    use crate::keymap::KeyCode;
    use crate::unsupported;

    pub(super) fn set_layer_key(_key_code: KeyCode) {}

    pub(super) fn run_event_loop(service_mode: bool) -> Result<()> {
        unsupported::run_event_loop(service_mode)
    }

    pub(super) fn start(layer_key: Option<KeyCode>) -> Result<()> {
        unsupported::start(layer_key)
    }

    pub(super) fn stop() -> Result<()> {
        unsupported::stop()
    }

    pub(super) fn restart(layer_key: Option<KeyCode>) -> Result<()> {
        unsupported::restart(layer_key)
    }

    pub(super) fn enable(layer_key: Option<KeyCode>) -> Result<()> {
        unsupported::enable(layer_key)
    }

    pub(super) fn disable() -> Result<()> {
        unsupported::disable()
    }

    pub(super) fn status() -> Result<()> {
        unsupported::status()
    }

    pub(super) fn request_permissions() -> Result<()> {
        unsupported::request_permissions()
    }
}

#[cfg(target_os = "windows")]
mod backend {
    use crate::error::Result;
    use crate::keymap::KeyCode;
    use crate::windows::{hook, permissions, service};

    pub(super) fn set_layer_key(key_code: KeyCode) {
        hook::set_layer_key(key_code);
    }

    pub(super) fn run_event_loop(service_mode: bool) -> Result<()> {
        hook::run_event_loop(service_mode)
    }

    pub(super) fn start(layer_key: Option<KeyCode>) -> Result<()> {
        service::start(layer_key)
    }

    pub(super) fn stop() -> Result<()> {
        service::stop()
    }

    pub(super) fn restart(layer_key: Option<KeyCode>) -> Result<()> {
        service::restart(layer_key)
    }

    pub(super) fn enable(layer_key: Option<KeyCode>) -> Result<()> {
        service::enable(layer_key)
    }

    pub(super) fn disable() -> Result<()> {
        service::disable()
    }

    pub(super) fn status() -> Result<()> {
        service::status()
    }

    pub(super) fn request_permissions() -> Result<()> {
        permissions::request_permissions()
    }
}
