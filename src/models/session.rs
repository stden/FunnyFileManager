//! Purpose: A stored connection — what Far's `NetBox` plugin calls a session —
//! with its protocol, how it logs in, and where it came from.
//!
//! State only: the file the sessions live in is [`crate::sessions`]' business,
//! and so is the `.netbox` importer. Every value is one of the checked types
//! in [`fields`], so a session is valid however it was made.
//!
//! A session holds no password, ever. A password is asked for when the
//! connection needs it and is never written: not to `sessions.json`, not to
//! a log, not to the history. That is why [`Auth::Password`] carries nothing.

use serde::{Deserialize, Serialize};

use super::RemotePath;

mod fields;
mod origin;

pub use fields::{Field, HostName, InvalidField, KeyFile, Port, Problem, SessionName, UserName};
pub use origin::{untag_legacy_origins, Origin};

/// How the remote host is spoken to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Protocol {
    #[default]
    Sftp,
    Scp,
    Ftp,
    Ftps,
}

impl Protocol {
    /// In the order the session dialog offers them.
    pub const ALL: [Self; 4] = [Self::Sftp, Self::Scp, Self::Ftp, Self::Ftps];

    #[must_use]
    pub const fn default_port(self) -> Port {
        match self {
            Self::Sftp | Self::Scp => Port::SSH,
            Self::Ftp | Self::Ftps => Port::FTP,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sftp => "SFTP",
            Self::Scp => "SCP",
            Self::Ftp => "FTP",
            Self::Ftps => "FTPS",
        }
    }

    /// The URL scheme a session's address is written with.
    #[must_use]
    pub const fn scheme(self) -> &'static str {
        match self {
            Self::Sftp => "sftp",
            Self::Scp => "scp",
            Self::Ftp => "ftp",
            Self::Ftps => "ftps",
        }
    }
}

/// How the session proves who it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Auth {
    /// A password, asked for each time the session connects.
    #[default]
    Password,
    /// A private key file.
    KeyFile(KeyFile),
    /// The running SSH agent.
    Agent,
}

impl Auth {
    #[must_use]
    pub const fn kind(&self) -> AuthKind {
        match self {
            Self::Password => AuthKind::Password,
            Self::KeyFile(_) => AuthKind::KeyFile,
            Self::Agent => AuthKind::Agent,
        }
    }
}

/// The shape of an [`Auth`] without its key file — what the dialog's radio
/// row chooses between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthKind {
    #[default]
    Password,
    KeyFile,
    Agent,
}

impl AuthKind {
    pub const ALL: [Self; 3] = [Self::Password, Self::KeyFile, Self::Agent];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::KeyFile => "Key file",
            Self::Agent => "Agent",
        }
    }
}

/// One stored connection. Every field is a checked value, so any
/// combination of them is a session this program can hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub name: SessionName,
    pub protocol: Protocol,
    pub host: HostName,
    pub port: Port,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<UserName>,
    #[serde(default)]
    pub auth: Auth,
    /// The directory the panel opens in; the login directory when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_dir: Option<RemotePath>,
    #[serde(
        default,
        skip_serializing_if = "Origin::is_manual",
        deserialize_with = "origin::read"
    )]
    pub origin: Origin,
}

impl Session {
    /// A session named `NetBox`'s way, with password login, no start
    /// directory, made by hand.
    #[must_use]
    pub fn new(protocol: Protocol, host: HostName, port: Port, user: Option<UserName>) -> Self {
        Self {
            name: SessionName::default_for(user.as_ref(), &host, port, protocol.default_port()),
            protocol,
            host,
            port,
            user,
            auth: Auth::default(),
            remote_dir: None,
            origin: Origin::default(),
        }
    }

    /// `user@host`, or the host alone — the panel's second column when it
    /// differs from the name.
    #[must_use]
    pub fn login(&self) -> String {
        self.user.as_ref().map_or_else(
            || self.host.to_string(),
            |user| format!("{user}@{}", self.host),
        )
    }

    /// `sftp://user@host:port/dir`, the port written only when it is not the
    /// protocol's own.
    #[must_use]
    pub fn url(&self) -> String {
        let port = if self.port == self.protocol.default_port() {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        let dir = self.remote_dir.as_ref().map_or("", RemotePath::as_str);
        format!("{}://{}{port}{dir}", self.protocol.scheme(), self.login())
    }
}

#[cfg(test)]
mod tests;
