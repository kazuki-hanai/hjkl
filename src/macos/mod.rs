//! macOS platform integration.
//!
//! Raw FFI stays in [`ffi`]; the other modules wrap it in narrowly scoped,
//! mostly safe helpers.

pub(crate) mod accessibility;
pub(crate) mod event;
pub(crate) mod event_tap;
pub(crate) mod ffi;
pub(crate) mod keys;
pub(crate) mod remapper;
pub(crate) mod service;
