//! Purpose: Own bounded, private JSON-line connections to headless FFM sessions.

pub mod connection;
mod error;
pub mod launch;
mod protocol;
mod registry;
mod types;

pub use error::Error;
pub use registry::SessionManager;
pub use types::{MouseInput, Profile, Request, SessionId, Size, Started};

#[cfg(test)]
mod tests;
