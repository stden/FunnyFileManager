//! Purpose: What a database request can fail with. Every message is safe to
//! show: it never holds a password.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DbError {
    /// The entry names an environment variable for its password, and it is unset.
    MissingPassword(String),
    /// The driver's own text, with any password taken out.
    Driver(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPassword(variable) => {
                write!(out, "environment variable {variable} is not set")
            }
            Self::Driver(text) => out.write_str(text),
        }
    }
}

#[cfg(test)]
mod tests;
