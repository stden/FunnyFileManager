//! Purpose: State for F9 Commands > Install MCP server: the scope radio, one
//! checkbox per AI client, and which control has the cursor.
//!
//! The survey that fills it is `ops::mcp_install`'s; the keys that move
//! through it are `popup::mcp_install`'s.

use crate::mcp_clients::{
    ClientId, ClientStatus, ConfigState, Presence, ProjectDir, Scope, ServerState, Survey,
};
use crate::search::Direction;

/// A client row's checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Checked,
    Unchecked,
}

impl Mark {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Checked => Self::Unchecked,
            Self::Unchecked => Self::Checked,
        }
    }
}

/// One client, as surveyed, and whether it will be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRow {
    pub status: ClientStatus,
    pub mark: Mark,
}

/// Whether Register has been pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpInstallProgress {
    /// The user is still choosing.
    Choosing,
    /// Register was pressed: the registration runs once a frame showing
    /// this message, what is about to run, has been drawn.
    Registering(String),
}

/// Which control has the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpInstallField {
    Scope,
    Client(ClientId),
    Register,
    Cancel,
}

/// The Install MCP server dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpInstallDialog {
    pub scope: Scope,
    /// The active panel's folder, when it is a local one.
    pub folder: Option<ProjectDir>,
    pub server: ServerState,
    pub rows: Vec<ClientRow>,
    pub field: McpInstallField,
    pub progress: McpInstallProgress,
}

impl McpInstallDialog {
    /// The dialog over `survey`, the cursor on the scope radio.
    #[must_use]
    pub fn new(survey: Survey) -> Self {
        let mut dialog = Self {
            scope: survey.scope,
            folder: None,
            server: ServerState::Missing,
            rows: Vec::new(),
            field: McpInstallField::Scope,
            progress: McpInstallProgress::Choosing,
        };
        dialog.apply(survey);
        dialog
    }

    /// Show a new survey, every mark reset to its defaults.
    pub fn apply(&mut self, survey: Survey) {
        let server = survey.server;
        self.rows = survey
            .clients
            .into_iter()
            .map(|status| ClientRow {
                mark: default_mark(&status, &server),
                status,
            })
            .collect();
        self.scope = survey.scope;
        self.folder = survey.folder;
        self.server = server;
        if !self.focus_order().contains(&self.field) {
            self.field = McpInstallField::Scope;
        }
    }

    /// Whether Register can do anything: there is a server to register.
    #[must_use]
    pub const fn can_register(&self) -> bool {
        self.server.binary().is_some()
    }

    /// Whether `row` can be toggled: there is a server, and the row's config
    /// would change. A registered row is shown checked and dimmed, because an
    /// unchecked one would read as "will be removed", and this dialog never
    /// removes anything.
    #[must_use]
    pub const fn is_enabled(&self, row: &ClientRow) -> bool {
        self.can_register() && row.status.state.is_actionable()
    }

    /// The controls Tab visits, in order: disabled ones are skipped.
    #[must_use]
    pub fn focus_order(&self) -> Vec<McpInstallField> {
        let clients = self
            .rows
            .iter()
            .filter(|row| self.is_enabled(row))
            .map(|row| McpInstallField::Client(row.status.client));
        std::iter::once(McpInstallField::Scope)
            .chain(clients)
            .chain(self.can_register().then_some(McpInstallField::Register))
            .chain([McpInstallField::Cancel])
            .collect()
    }

    /// Tab / Shift+Tab and the vertical arrows: the next control, wrapping.
    pub fn advance(&mut self, direction: Direction) {
        self.field = crate::cycle::step(&self.focus_order(), self.field, !direction.is_backward());
    }

    /// Space: flip the focused client's mark. Nothing else answers to it.
    pub fn toggle(&mut self) {
        let McpInstallField::Client(client) = self.field else {
            return;
        };
        let enabled = self
            .rows
            .iter()
            .any(|row| row.status.client == client && self.is_enabled(row));
        if let Some(row) = self
            .rows
            .iter_mut()
            .find(|row| enabled && row.status.client == client)
        {
            row.mark = row.mark.toggled();
        }
    }

    /// The clients Register works through, in table order. Registered rows
    /// are among them, and come back "unchanged".
    #[must_use]
    pub fn checked_clients(&self) -> Vec<ClientId> {
        self.rows
            .iter()
            .filter(|row| row.mark == Mark::Checked)
            .map(|row| row.status.client)
            .collect()
    }

    /// Where the scope sits among the radio's options.
    #[must_use]
    pub fn scope_index(&self) -> usize {
        Scope::ALL
            .iter()
            .position(|scope| *scope == self.scope)
            .unwrap_or_default()
    }
}

/// A row's mark when the dialog opens or the scope changes.
fn default_mark(status: &ClientStatus, server: &ServerState) -> Mark {
    // Without a server nothing will be written, so only what is already done
    // is shown as checked.
    let checked = match (&status.state, server.binary()) {
        (ConfigState::Registered | ConfigState::Disabled, _)
        | (ConfigState::DifferentEntry, Some(_)) => true,
        (_, None)
        | (
            ConfigState::Blocked(_) | ConfigState::Unverified { .. } | ConfigState::ProgramMissing,
            _,
        ) => false,
        (ConfigState::NoConfig | ConfigState::NotConfigured, Some(_)) => {
            status.presence == Presence::Detected
        }
    };
    if checked {
        Mark::Checked
    } else {
        Mark::Unchecked
    }
}

#[cfg(test)]
mod tests;
