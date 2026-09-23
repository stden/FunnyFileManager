//! Purpose: The remote filesystem a stored session opens, as one trait, and
//! the shared handle a remote panel holds it by.
//!
//! The remote panel is written against [`RemoteFs`], so a fake in a test and
//! a network transport are the same thing to it. This build has no network
//! transport: [`connect`] says so for every session, and the panel is reached
//! through [`crate::ops::remote::open_with`] with the in-memory
//! [`fake::FakeRemoteFs`] in tests. Listings are the panel's own [`DirEntry`],
//! so a remote directory is drawn by the same rules as a local one.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{DirEntry, Protocol, RemotePath, Session};

#[cfg(any(test, feature = "test-helpers"))]
pub mod fake;

/// Why a remote operation did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// This build carries no transport for the protocol.
    NoTransport(Protocol),
    NotFound(RemotePath),
    NotADirectory(RemotePath),
}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTransport(protocol) => write!(
                f,
                "this build cannot connect over {}; the session is stored for when it can",
                protocol.label()
            ),
            Self::NotFound(path) => write!(f, "no such remote directory: {path}"),
            Self::NotADirectory(path) => write!(f, "not a directory: {path}"),
        }
    }
}

impl std::error::Error for RemoteError {}

/// What the remote panel needs from a host.
pub trait RemoteFs: Send {
    /// The entries of the directory `dir`, without `.` and `..`.
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError>;
}

/// An open connection, shared by every copy of the panel that holds it.
///
/// Shared rather than owned because a panel is cloned and the clone written
/// back — the settings dialog stages both panels that way — and a clone that
/// dropped or duplicated the connection would disconnect the panel or open a
/// second session to the host. A `Mutex` rather than a `RefCell` keeps `App`
/// `Send`. Equality is identity, and `Debug` names no host.
#[derive(Clone)]
pub struct RemoteLink(Arc<Mutex<Box<dyn RemoteFs>>>);

impl RemoteLink {
    #[must_use]
    pub fn new(fs: Box<dyn RemoteFs>) -> Self {
        Self(Arc::new(Mutex::new(fs)))
    }

    /// The connection, for one operation. A poisoned lock is recovered: a
    /// listing holds no invariant that a panic halfway through could break.
    pub fn lock(&self) -> MutexGuard<'_, Box<dyn RemoteFs>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for RemoteLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RemoteLink(<connected>)")
    }
}

impl PartialEq for RemoteLink {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for RemoteLink {}

/// Open `session`'s host. This build has no transport for any protocol, so
/// the answer is always [`RemoteError::NoTransport`]; the stored session is
/// unaffected.
pub fn connect(session: &Session) -> Result<Box<dyn RemoteFs>, RemoteError> {
    Err(RemoteError::NoTransport(session.protocol))
}

#[cfg(test)]
mod tests;
