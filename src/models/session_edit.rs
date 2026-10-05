//! Purpose: The session dialog (`F4` / `Shift+F4` on the sessions panel):
//! every field of a stored session as typed text, and which control has the
//! cursor.
//!
//! State only: the keys that edit it live in `popup::session_edit`, the frame
//! in `ui::session_edit`, and storing the answer in `ops::sessions`. Turning
//! the typed text into a [`Session`] is [`SessionEdit::to_session`], here, so
//! the dialog and the store never disagree about what a valid session is.
//! There is no password field: a password is asked for when connecting.

use super::{
    Auth, AuthKind, HostName, KeyFile, Origin, Port, Protocol, RemotePath, Session, SessionName,
    UserName,
};

mod controls;

/// The controls, in the order Tab walks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionField {
    #[default]
    Name,
    Protocol,
    Host,
    Port,
    User,
    Auth,
    KeyFile,
    RemoteDir,
    Ok,
    Cancel,
}

impl SessionField {
    pub const ALL: [Self; 10] = [
        Self::Name,
        Self::Protocol,
        Self::Host,
        Self::Port,
        Self::User,
        Self::Auth,
        Self::KeyFile,
        Self::RemoteDir,
        Self::Ok,
        Self::Cancel,
    ];
}

/// Whether the dialog makes a session or changes one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditPurpose {
    /// `Shift+F4`: a new session.
    #[default]
    Create,
    /// `F4`: the session stored under this name; a rename replaces it.
    Edit(SessionName),
}

/// The dialog's fields as typed, and the control with the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionEdit {
    /// Empty means `NetBox`'s default name, `user@host`.
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    /// Empty means the protocol's own port.
    pub port: String,
    /// Empty means no user name.
    pub user: String,
    pub auth: AuthKind,
    pub key_file: String,
    /// Empty means the login directory.
    pub remote_dir: String,
    field: SessionField,
    purpose: EditPurpose,
}

impl SessionEdit {
    /// The empty dialog for `Shift+F4`, its port field showing SFTP's own.
    #[must_use]
    pub fn create() -> Self {
        Self {
            port: Protocol::default().default_port().to_string(),
            ..Self::default()
        }
    }

    /// The dialog filled from a stored session, for `F4`.
    #[must_use]
    pub fn from_session(session: &Session) -> Self {
        let key_file = match &session.auth {
            Auth::KeyFile(file) => file.to_string(),
            Auth::Password | Auth::Agent => String::new(),
        };
        Self {
            name: session.name.to_string(),
            protocol: session.protocol,
            host: session.host.to_string(),
            port: session.port.to_string(),
            user: session
                .user
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            auth: session.auth.kind(),
            key_file,
            remote_dir: session
                .remote_dir
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            field: SessionField::default(),
            purpose: EditPurpose::Edit(session.name.clone()),
        }
    }

    #[must_use]
    pub const fn field(&self) -> SessionField {
        self.field
    }

    #[must_use]
    pub const fn purpose(&self) -> &EditPurpose {
        &self.purpose
    }

    /// The session the dialog describes, or the first field that is wrong.
    /// A session saved from the dialog is the user's own, whatever it was
    /// imported from, so a later import does not overwrite the change.
    pub fn to_session(&self) -> Result<Session, super::InvalidField> {
        let host = HostName::new(&self.host)?;
        let port = if self.port.trim().is_empty() {
            self.protocol.default_port()
        } else {
            Port::parse(&self.port)?
        };
        let user = optional(&self.user, UserName::new)?;
        let name = match optional(&self.name, SessionName::new)? {
            Some(name) => name,
            None => {
                SessionName::default_for(user.as_ref(), &host, port, self.protocol.default_port())
            }
        };
        Ok(Session {
            name,
            protocol: self.protocol,
            host,
            port,
            user,
            auth: self.auth()?,
            remote_dir: optional(&self.remote_dir, RemotePath::parse)?,
            origin: Origin::Manual,
        })
    }

    fn auth(&self) -> Result<Auth, super::InvalidField> {
        Ok(match self.auth {
            AuthKind::Password => Auth::Password,
            AuthKind::KeyFile => Auth::KeyFile(KeyFile::new(&self.key_file)?),
            AuthKind::Agent => Auth::Agent,
        })
    }
}

/// A field that may be left blank: `None` when it is, else its check.
fn optional<T, E>(text: &str, check: impl FnOnce(&str) -> Result<T, E>) -> Result<Option<T>, E> {
    if text.trim().is_empty() {
        Ok(None)
    } else {
        check(text).map(Some)
    }
}

#[cfg(test)]
mod tests;
