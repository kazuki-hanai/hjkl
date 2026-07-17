//! Application error type.
//!
//! Every failure in this crate is ultimately a message shown to the user, so
//! the error type is a thin wrapper around that message. Keeping it as a
//! dedicated type (instead of a bare `String`) gives `?` conversions a single
//! place to hang off and leaves room for richer variants later.

use std::fmt;

/// A user-facing error message.
#[derive(Debug)]
pub struct Error(String);

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self(message.to_string())
    }
}
