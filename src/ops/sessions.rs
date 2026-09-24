//! Purpose: `NetBox`'s stored-sessions panel — opening it, leaving it, and
//! changing the stored sessions from it: the dialog, a copy, a delete, the
//! details view. Importing is the child module [`import`].
//!
//! Every change goes through `change` in [`changes`]: it edits a copy of the
//! store, writes the copy, and only then puts it in place, so a session the
//! file did not take never shows in the panel as if it had been saved.

use std::{fmt, io};

use crate::command::SessionsCommand;

use crate::models::{App, InvalidField, PanelKind, PanelSide, Session, SessionName, SessionsPanel};
use crate::sessions::import::ImportError;
use crate::sessions::{SessionStore, StoreError};
use crate::status::Status;

mod changes;
mod details;
pub mod import;

pub use changes::{create, delete, duplicate, edit, save_edit};
// `import` saves through the same `change`.
use changes::change;
pub use details::view;

/// Why a sessions operation did not happen.
#[derive(Debug)]
pub enum SessionError {
    NotFound(SessionName),
    NameTaken(SessionName),
    Invalid(InvalidField),
    Store(StoreError),
    Import(ImportError),
    /// No file was named, and no file panel's cursor is on a `.netbox` file.
    NoImportSource,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(name) => write!(f, "no session named {name}"),
            Self::NameTaken(name) => write!(f, "a session named {name} already exists"),
            Self::Invalid(field) => write!(f, "{field}"),
            Self::Store(error) => write!(f, "{error}"),
            Self::Import(error) => write!(f, "{error}"),
            Self::NoImportSource => {
                f.write_str("name a .netbox file, or put a file panel's cursor on one")
            }
        }
    }
}

impl std::error::Error for SessionError {}

impl From<InvalidField> for SessionError {
    fn from(field: InvalidField) -> Self {
        Self::Invalid(field)
    }
}

/// A failed operation's one line in the status bar.
pub fn report(app: &mut App, outcome: Result<(), SessionError>) {
    if let Err(error) = outcome {
        app.set_status(Status::Error(format!("NetBox: {error}")));
    }
}

/// Put the store bootstrap loaded in place, and say on the status bar when
/// its file did not load — every change will be refused until it is fixed.
pub fn install(app: &mut App, store: SessionStore) {
    app.sessions = store;
    if app.sessions.is_damaged() {
        report(app, Err(SessionError::Store(StoreError::Damaged)));
    }
}

/// `:sessions …`: open the panel, or import. An import that fails is an
/// error of the command, reported the way a failed command is.
pub fn execute(app: &mut App, command: SessionsCommand) -> io::Result<()> {
    match command {
        SessionsCommand::Open => {
            open(app);
            Ok(())
        }
        SessionsCommand::ImportNetBox(path) => {
            import::run(app, path).map_err(|error| io::Error::other(format!("NetBox: {error}")))
        }
    }
}

/// `:sessions` and the F11 row: the active panel becomes the stored-sessions
/// list.
pub fn open(app: &mut App) {
    let status = open_on(app, app.active_panel);
    app.set_status(status);
}

/// The same on `side`'s panel — the Change drive dialog names its side —
/// and what the status bar should say about it: how many sessions there
/// are, or that the file did not load.
#[must_use]
pub fn open_on(app: &mut App, side: PanelSide) -> Status {
    let panel = SessionsPanel::from_store(&app.sessions);
    let count = panel.session_count();
    app.panel_mut(side).kind = PanelKind::Sessions(Box::new(panel));
    if app.sessions.is_damaged() {
        return Status::Error(format!("NetBox: {}", StoreError::Damaged));
    }
    let plural = if count == 1 { "" } else { "s" };
    Status::CommandOutput(format!("NetBox: {count} stored session{plural}"))
}

/// Enter on `..`: the active panel shows its files again. A remote panel's
/// connection goes with it.
pub fn back_to_files(app: &mut App) {
    back_to_files_on(app, app.active_panel);
}

/// The same on `side`'s panel — a local drive picked in the Change drive
/// dialog must not leave the sessions list or a host drawn over it.
pub fn back_to_files_on(app: &mut App, side: PanelSide) {
    let panel = app.panel_mut(side);
    if panel.kind.is_netbox() {
        panel.kind = PanelKind::Files;
    }
}

/// Rebuild every sessions panel from the store, after a change to it.
pub fn refresh(app: &mut App) {
    for side in [PanelSide::Left, PanelSide::Right] {
        let store = &app.sessions;
        let panel = match side {
            PanelSide::Left => &mut app.left_panel,
            PanelSide::Right => &mut app.right_panel,
        };
        if let PanelKind::Sessions(sessions) = &mut panel.kind {
            sessions.reload(store);
        }
    }
}

fn find<'a>(app: &'a App, name: &SessionName) -> Result<&'a Session, SessionError> {
    app.sessions
        .find(name)
        .ok_or_else(|| SessionError::NotFound(name.clone()))
}

#[cfg(test)]
mod tests;
