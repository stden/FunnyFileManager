//! Purpose: One row of the client table — the `Client` type — and the scope,
//! format and mechanism its fields and methods speak in.

use super::ClientId;

/// Where a registration applies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scope {
    /// The client's user-level config: every project it opens.
    #[default]
    AllProjects,
    /// The active panel's folder: a project-level config inside it.
    CurrentFolder,
}

impl Scope {
    /// The radio's options, in order.
    pub const ALL: [Self; 2] = [Self::AllProjects, Self::CurrentFolder];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AllProjects => "All projects",
            Self::CurrentFolder => "Current folder",
        }
    }

    /// The other scope: there are two, so both arrow keys land here.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::AllProjects => Self::CurrentFolder,
            Self::CurrentFolder => Self::AllProjects,
        }
    }
}

/// How a client's config file is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// `{"mcpServers": {"ffm-mcp": {…}}}`.
    Json,
    /// `[mcp_servers.ffm-mcp]`.
    Toml,
}

/// How a user-level registration reaches the client's config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    /// FFM edits the file itself.
    EditFile,
    /// `claude mcp add`: `~/.claude.json` also holds the sign-in session, so
    /// nothing but Claude Code writes it.
    ClaudeCli,
}

/// One row of the client table.
#[derive(Debug, PartialEq, Eq)]
pub struct Client {
    pub id: ClientId,
    pub name: &'static str,
    /// Looked up on PATH: any one found means "detected", and the first found
    /// is the program run when `user_level` is [`Mechanism::ClaudeCli`].
    pub programs: &'static [&'static str],
    /// The variable that moves the client's home folder, if it has one.
    pub home_var: Option<&'static str>,
    /// The client's home folder under the user's home when `home_var` is
    /// unset; "" is the user's home itself.
    pub home_dir: &'static str,
    /// The user-level config, relative to the client's home folder.
    pub user_config: &'static str,
    /// The project-level config, relative to the folder.
    pub project_config: &'static str,
    pub format: ConfigFormat,
    /// How a user-level registration is written. Project level is always
    /// [`Mechanism::EditFile`].
    pub user_level: Mechanism,
    pub user_note: Option<&'static str>,
    pub project_note: Option<&'static str>,
}

impl Client {
    /// The config relative to its base folder at `scope`, `/`-separated.
    #[must_use]
    pub const fn config(&self, scope: Scope) -> &'static str {
        match scope {
            Scope::AllProjects => self.user_config,
            Scope::CurrentFolder => self.project_config,
        }
    }

    /// What the dialog and the report say about this client at `scope`.
    #[must_use]
    pub const fn note(&self, scope: Scope) -> Option<&'static str> {
        match scope {
            Scope::AllProjects => self.user_note,
            Scope::CurrentFolder => self.project_note,
        }
    }

    /// How a registration at `scope` is written.
    #[must_use]
    pub const fn mechanism(&self, scope: Scope) -> Mechanism {
        match scope {
            Scope::AllProjects => self.user_level,
            Scope::CurrentFolder => Mechanism::EditFile,
        }
    }

    /// The note as the dialog and the report print it: `<name>: <note>`.
    #[must_use]
    pub fn note_line(&self, scope: Scope) -> Option<String> {
        self.note(scope)
            .map(|note| format!("{}: {note}", self.name))
    }
}
