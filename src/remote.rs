//! Purpose: The remote filesystem a stored session opens, as one trait, and
//! the shared handle a remote panel holds it by.
//!
//! Native SFTP and FTP adapters share the same interface as isolated test hosts.
//! Listings use the panel domain types. Credentials are transient; SSH host
//! identity is checked before authentication.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::models::{Protocol, RemotePath, Session};

mod connection;
mod filesystem;
mod ftp;
mod metadata;
mod sftp;
pub use connection::{connect_with, Credentials};
pub use filesystem::RemoteFs;
pub use metadata::{RemoteMetadata, UnixMode, WriteMode};

#[cfg(any(test, feature = "test-helpers"))]
pub mod fake;

/// Why a remote operation did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// This build carries no transport for the protocol.
    NoTransport(Protocol),
    NotFound(RemotePath),
    NotADirectory(RemotePath),
    AlreadyExists(RemotePath),
    Unsupported(&'static str),
    InvalidInput(&'static str),
    Failed(&'static str),
    AuthenticationRequired,
    HostKey(String),
    Conflict,
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
            Self::AlreadyExists(path) => write!(f, "remote destination already exists: {path}"),
            Self::Unsupported(operation) => write!(f, "remote server does not support {operation}"),
            Self::InvalidInput(message) | Self::Failed(message) => f.write_str(message),
            Self::AuthenticationRequired => f.write_str("enter the remote username and password"),
            Self::HostKey(fingerprint) => write!(f, "SSH host key is not trusted: {fingerprint}"),
            Self::Conflict => f.write_str("the remote file changed since it was opened"),
        }
    }
}

impl std::error::Error for RemoteError {}

impl From<std::io::Error> for RemoteError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;
        Self::Failed(match error.kind() {
            ErrorKind::PermissionDenied => "remote operation: permission denied",
            ErrorKind::TimedOut | ErrorKind::WouldBlock => "remote operation timed out",
            ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => "remote connection closed",
            _ => "remote file transfer failed",
        })
    }
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
    if matches!(session.protocol, Protocol::Sftp | Protocol::Ftp)
        && matches!(session.auth, crate::models::Auth::Password)
    {
        return Err(RemoteError::AuthenticationRequired);
    }
    connect_with(session, &Credentials::default())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_operations;
