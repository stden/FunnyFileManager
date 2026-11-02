//! Purpose: The Things asset catalog: its machines, read with the user's token
//! over HTTPS, and the SFTP sessions they become.
//!
//! Things is the user's internal catalog. Its address is in `THINGS_URL` and
//! its token in `THINGS_API_TOKEN`, read from the environment when the catalog
//! dialog opens. Neither is stored: the address and token live in the dialog
//! while it is open, and an imported machine keeps only its id, as its
//! session's origin.

mod client;
mod config;
mod filter;
mod listing;
mod machine;
mod mapping;

#[cfg(test)]
mod tests;

pub use client::fetch;
pub use config::{Config, ConfigError, Token, TOKEN_VARIABLE, URL_VARIABLE};
pub use filter::matches;
pub use listing::{parse, Listing};
pub use machine::{Machine, OsInfo, Status};
pub use mapping::sessions_from;

/// Why the catalog could not be read. It carries neither the address nor the
/// token; each message says what to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThingsError {
    /// Reads are switched off here: a test, a replay or an isolated session.
    Disabled,
    /// The request failed, or the server refused it.
    Request,
    /// The body is not a machine list.
    Response,
    /// The body is larger than a machine list may be.
    Limit,
    /// The worker thread could not run, or ended without an answer.
    Worker,
}

impl ThingsError {
    /// The line the dialog shows for this failure.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Disabled => "Unavailable in replay or isolated sessions",
            Self::Request => "Things request failed; check THINGS_URL, the token and the network",
            Self::Response => "Things returned an unexpected response",
            Self::Limit => "Things returned more data than a machine list may hold",
            Self::Worker => "Things worker failed",
        }
    }
}
