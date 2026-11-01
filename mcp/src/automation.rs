//! Purpose: Own bounded, private JSON-line connections to headless FFM sessions.

mod connection;
mod error;
mod launch;
mod protocol;
mod registry;
mod types;

pub use error::Error;
pub use registry::SessionManager;
pub use types::{MouseInput, Profile, Request, SessionId, Size, Started};

#[cfg(test)]
mod tests;
