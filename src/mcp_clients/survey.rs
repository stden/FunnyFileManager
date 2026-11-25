//! Purpose: What the survey found on the machine, and the words the dialog
//! and the `:mcp` screen print for it: a client's row and the server's lines.

use std::path::PathBuf;

use super::{ClientId, ConfigState, ProjectDir, Scope, ServerState, SERVER_NAME};
use crate::status::PathMessage;

/// Whether the client looks installed on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Detected,
    NotDetected,
}

impl Presence {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::NotDetected => "not detected",
        }
    }
}

/// How to get the server when it is missing.
pub const INSTALL_HINT: &str = "Install it from the FFM sources: cargo install --path mcp";

/// What the dialog and the `:mcp` screen say about the server, each line
/// split around the path it quotes: a dialog row too narrow for it shortens
/// the path and keeps the server's name.
#[must_use]
pub fn server_lines(server: &ServerState) -> Vec<PathMessage<'_>> {
    match server {
        ServerState::Found(binary) => vec![PathMessage::quoting(
            SERVER_CAPTION,
            crate::path_display::label(binary.as_path()),
            "",
        )],
        ServerState::Missing => vec![
            PathMessage::from(format!(
                "{SERVER_CAPTION}{SERVER_NAME} was not found next to ffm or on PATH."
            )),
            PathMessage::from(INSTALL_HINT.to_owned()),
        ],
        ServerState::NotUnicode(path) => vec![
            PathMessage::quoting(
                SERVER_CAPTION,
                path.to_string_lossy(),
                " is not valid Unicode, so no config can name it.",
            ),
            PathMessage::from(INSTALL_HINT.to_owned()),
        ],
    }
}

/// What every line naming the server starts with.
const SERVER_CAPTION: &str = "Server: ";

/// One client's row in the dialog: name, presence, state, in columns.
#[must_use]
pub fn row_label(status: &ClientStatus) -> String {
    format!(
        "{:<12} {:<13} {}",
        status.client.name(),
        status.presence.label(),
        status.state.label()
    )
}

/// One client as the survey found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientStatus {
    pub client: ClientId,
    pub presence: Presence,
    /// The config file for the surveyed scope; `None` when its folder cannot
    /// be worked out (no home folder, or an override that is not a folder).
    pub config: Option<PathBuf>,
    pub state: ConfigState,
}

/// What the machine looks like for one scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survey {
    pub server: ServerState,
    /// The active panel's folder, when it is a local folder.
    pub folder: Option<ProjectDir>,
    /// Never [`Scope::CurrentFolder`] without a `folder`.
    pub scope: Scope,
    pub clients: Vec<ClientStatus>,
}
