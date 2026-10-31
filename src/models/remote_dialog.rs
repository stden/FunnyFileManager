//! Purpose: Remote credentials, file-operation inputs and confirmations are modal state.

use super::remote_work::{FileLocation, RemoteTarget};
use super::{Confirm, Session};
use crate::lineedit::LineEditor;
use crate::remote::Credentials;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    User,
    Password,
    Connect,
    Cancel,
}

impl LoginField {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::User => Self::Password,
            Self::Password => Self::Connect,
            Self::Connect => Self::Cancel,
            Self::Cancel => Self::User,
        }
    }
}

/// Password editing never passes through history, ordinary text fields or Debug.
#[derive(Clone)]
pub struct RemoteLogin {
    pub session: Session,
    pub user: LineEditor,
    pub credentials: Credentials,
    pub field: LoginField,
    pub notice: String,
}

impl RemoteLogin {
    pub(crate) fn new(session: Session) -> Self {
        let user = session
            .user
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        Self {
            session,
            user: LineEditor::new(&user),
            credentials: Credentials::default(),
            field: if user.is_empty() {
                LoginField::User
            } else {
                LoginField::Password
            },
            notice: String::new(),
        }
    }
}

#[derive(Clone)]
pub enum RemoteInput {
    Transfer {
        sources: Vec<FileLocation>,
        directory: FileLocation,
        moving: bool,
    },
    Rename(FileLocation),
    Mkdir(FileLocation),
    Chmod(Vec<FileLocation>),
    Selection {
        select: bool,
    },
}

impl RemoteInput {
    pub(crate) const fn title(&self) -> &'static str {
        match self {
            Self::Transfer { moving: true, .. } => "Move",
            Self::Transfer { .. } => "Copy",
            Self::Rename(_) => "Rename",
            Self::Mkdir(_) => "Make directory",
            Self::Chmod(_) => "Linux permissions",
            Self::Selection { select: true } => "Select files",
            Self::Selection { .. } => "Deselect files",
        }
    }
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Transfer { .. } => "Destination directory:",
            Self::Rename(_) => "New name:",
            Self::Mkdir(_) => "Directory name:",
            Self::Chmod(_) => "Permissions (octal, for example 0644 or 0755):",
            Self::Selection { .. } => "File mask:",
        }
    }
}

#[derive(Clone)]
pub enum RemoteDialog {
    Login(RemoteLogin),
    Trust {
        login: RemoteLogin,
        fingerprint: String,
        answer: Confirm,
    },
    Input {
        target: RemoteTarget,
        purpose: RemoteInput,
        input: LineEditor,
        cancel: bool,
        notice: String,
    },
    Delete {
        target: RemoteTarget,
        sources: Vec<FileLocation>,
        answer: Confirm,
    },
}

/// Public overlay handle; its credential-bearing state stays private to the app.
#[derive(Clone)]
pub struct RemoteOverlay(pub(crate) RemoteDialog);
