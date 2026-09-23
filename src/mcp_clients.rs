//! Purpose: The AI clients FFM registers its MCP server with, in one table.
//!
//! The Install MCP server dialog, the installer and the `:mcp` screen all read
//! that table; the text-to-text logic that reads and edits the clients' config
//! files lives here too.
//!
//! Pure: nothing here touches the filesystem, starts a process or reads the
//! environment. `ops::mcp_install` does that, and reaches the machine only
//! through [`Host`], so tests hand it a fake one.

mod codex_toml;
mod entry;
mod host;
mod json;
mod state;

use std::path::PathBuf;

pub use entry::{ProjectDir, ServerBinary, ServerEntry, ServerState};
pub use host::{Captured, CliCall, Exit, Host, Runner};
pub use state::{Blocker, ConfigState};

/// The name every client knows the server by.
pub const SERVER_NAME: &str = "ffm-mcp";

/// The variable that confines the server to a folder.
///
/// A copy of `mcp/src/paths.rs::ROOT_ENV`: the TUI may not compile that file
/// (`tests/module_boundaries/mcp.rs`), so that test reads it as text and
/// checks the two still agree.
pub const ROOT_ENV: &str = "FFM_MCP_ROOT";

/// Set when Claude Code signs in through a custom OAuth server; its user
/// config is then another file (`ops::mcp_install::survey::user_config`).
pub const CLAUDE_CUSTOM_OAUTH_VAR: &str = "CLAUDE_CODE_CUSTOM_OAUTH_URL";

/// Every environment variable the installer reads, besides the home and
/// PATH: each client's `home_var`, and Claude Code's OAuth switch.
#[must_use]
pub fn env_vars() -> Vec<&'static str> {
    CLIENTS
        .iter()
        .filter_map(|client| client.home_var)
        .chain([CLAUDE_CUSTOM_OAUTH_VAR])
        .collect()
}

/// One AI client, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientId {
    ClaudeCode,
    Codex,
    Junie,
    Kiro,
}

impl ClientId {
    /// The client's row in [`CLIENTS`].
    #[must_use]
    pub const fn client(self) -> &'static Client {
        match self {
            Self::ClaudeCode => &CLAUDE_CODE,
            Self::Codex => &CODEX,
            Self::Junie => &JUNIE,
            Self::Kiro => &KIRO,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.client().name
    }
}

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

/// Every client's note for `scope`, in table order: what the dialog shows
/// under the rows, the same whichever rows are checked.
#[must_use]
pub fn notes(scope: Scope) -> Vec<String> {
    CLIENTS
        .iter()
        .filter_map(|client| client.note_line(scope))
        .collect()
}

/// How to get the server when it is missing.
pub const INSTALL_HINT: &str = "Install it from the FFM sources: cargo install --path mcp";

/// What the dialog and the `:mcp` screen say about the server.
#[must_use]
pub fn server_lines(server: &ServerState) -> Vec<String> {
    match server {
        ServerState::Found(binary) => vec![format!(
            "Server: {}",
            crate::path_display::label(binary.as_path())
        )],
        ServerState::Missing => vec![
            format!("Server: {SERVER_NAME} was not found next to ffm or on PATH."),
            INSTALL_HINT.to_owned(),
        ],
        ServerState::NotUnicode(path) => vec![
            format!(
                "Server: {} is not valid Unicode, so no config can name it.",
                path.display()
            ),
            INSTALL_HINT.to_owned(),
        ],
    }
}

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

/// Junie and Kiro may start a user-level server outside the project, where it
/// confines itself to the wrong folder.
const OUTSIDE_THE_PROJECT: &str = "may start ffm-mcp outside the project; prefer Current folder.";

const CLAUDE_CODE: Client = Client {
    id: ClientId::ClaudeCode,
    name: "Claude Code",
    programs: &["claude"],
    home_var: Some("CLAUDE_CONFIG_DIR"),
    home_dir: "",
    user_config: ".claude.json",
    project_config: ".mcp.json",
    format: ConfigFormat::Json,
    user_level: Mechanism::ClaudeCli,
    user_note: None,
    project_note: Some("asks once before it starts a server from .mcp.json."),
};

const CODEX: Client = Client {
    id: ClientId::Codex,
    name: "Codex",
    programs: &["codex"],
    home_var: Some("CODEX_HOME"),
    home_dir: ".codex",
    user_config: "config.toml",
    project_config: ".codex/config.toml",
    format: ConfigFormat::Toml,
    user_level: Mechanism::EditFile,
    user_note: None,
    project_note: Some("reads .codex/config.toml only in folders it trusts."),
};

const JUNIE: Client = Client {
    id: ClientId::Junie,
    name: "Junie",
    programs: &["junie"],
    home_var: None,
    home_dir: ".junie",
    user_config: "mcp/mcp.json",
    project_config: ".junie/mcp/mcp.json",
    format: ConfigFormat::Json,
    user_level: Mechanism::EditFile,
    user_note: Some(OUTSIDE_THE_PROJECT),
    project_note: None,
};

const KIRO: Client = Client {
    id: ClientId::Kiro,
    name: "Kiro",
    programs: &["kiro", "kiro-cli"],
    home_var: Some("KIRO_HOME"),
    home_dir: ".kiro",
    user_config: "settings/mcp.json",
    project_config: ".kiro/settings/mcp.json",
    format: ConfigFormat::Json,
    user_level: Mechanism::EditFile,
    user_note: Some(OUTSIDE_THE_PROJECT),
    project_note: None,
};

/// Every client, in the order the dialog, the report and `:mcp` list them.
/// The only list of clients in the crate: a new client in one of these
/// formats is one more row.
pub const CLIENTS: [Client; 4] = [CLAUDE_CODE, CODEX, JUNIE, KIRO];

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

/// The state of a config's text for `desired`. When the server was not found
/// (`None`) there is nothing to compare an entry with, so an entry is only
/// [`ConfigState::Unverified`].
#[must_use]
pub fn state(format: ConfigFormat, text: &str, desired: Option<&ServerEntry>) -> ConfigState {
    let found = match format {
        ConfigFormat::Json => json::entry(text),
        ConfigFormat::Toml => codex_toml::entry(text),
    };
    match found {
        Err(blocker) => ConfigState::Blocked(blocker),
        Ok(None) => ConfigState::NotConfigured,
        Ok(Some(found)) => found.state(desired),
    }
}

/// `text` with the `ffm-mcp` entry set to `desired`, every other key kept.
pub fn upsert(format: ConfigFormat, text: &str, desired: &ServerEntry) -> Result<String, Blocker> {
    match format {
        ConfigFormat::Json => json::upsert(text, desired),
        ConfigFormat::Toml => codex_toml::upsert(text, desired),
    }
}

/// The entry `claude mcp add-json` is given to replace Claude Code's.
///
/// It is the `ffm-mcp` entry in Claude Code's user config (`text`) with
/// `desired` set, exactly as [`upsert`] would leave it, so its `env` and
/// other keys survive.
pub fn claude_entry(text: &str, desired: &ServerEntry) -> Result<String, Blocker> {
    json::entry_text(text, desired)
}

/// The `ffm-mcp` entry a config holds, read out of either format.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FoundEntry {
    command: Option<String>,
    args: Args,
    root: Option<String>,
    launch: Launch,
    switch: Switch,
}

/// Whether an entry passes the server any arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Args {
    /// Absent or `[]`.
    Empty,
    Given,
}

/// Whether the client starts the entry the way FFM writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Launch {
    /// A local program, started in whatever folder the client picks.
    AsWritten,
    /// A `url`, a `cwd`, or a `type` other than `stdio`: not this server, or
    /// not confined where the scope says.
    Elsewhere,
}

/// Whether the client starts the entry at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Switch {
    On,
    /// `"disabled": true` or `enabled = false`, in either format.
    Off,
}

/// The keys that start a server some other way than FFM writes it, and so
/// go when the entry is set: `type` only when it is not `stdio`.
const LAUNCH_KEYS: [&str; 3] = ["url", "cwd", "type"];

/// The one `type` an entry may keep.
const STDIO: &str = "stdio";

impl FoundEntry {
    /// Every other key in the entry, and in its `env`, is somebody else's.
    fn matches(&self, desired: &ServerEntry) -> bool {
        self.command.as_deref() == Some(desired.command.as_str())
            && self.args == Args::Empty
            && self.launch == Launch::AsWritten
            && self.root.as_deref() == desired.root.as_ref().map(ProjectDir::as_str)
    }

    fn state(self, desired: Option<&ServerEntry>) -> ConfigState {
        match desired {
            None => ConfigState::Unverified {
                command: self.command,
            },
            Some(desired) if !self.matches(desired) => ConfigState::DifferentEntry,
            Some(_) if self.switch == Switch::Off => ConfigState::Disabled,
            Some(_) => ConfigState::Registered,
        }
    }
}

impl Launch {
    /// `Elsewhere` when any launch key would go on a write. `text` reads one
    /// key: `None` when it is absent, `Some(None)` when it is not text.
    fn read<'a>(text: impl Fn(&str) -> Option<Option<&'a str>>) -> Self {
        let dropped = LAUNCH_KEYS
            .iter()
            .any(|key| text(key).is_some_and(|value| drops_launch_key(key, value)));
        if dropped {
            Self::Elsewhere
        } else {
            Self::AsWritten
        }
    }
}

impl Switch {
    /// `Off` when `disabled` is `true` or `enabled` is `false`; `flag` reads
    /// one key as a boolean.
    fn read(flag: impl Fn(&str) -> Option<bool>) -> Self {
        if flag("disabled") == Some(true) || flag("enabled") == Some(false) {
            Self::Off
        } else {
            Self::On
        }
    }
}

/// Whether the entry's launch key `key`, holding `text` (when it is text),
/// goes when the entry is set.
fn drops_launch_key(key: &str, text: Option<&str>) -> bool {
    key != "type" || text != Some(STDIO)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_json;
#[cfg(test)]
mod tests_toml;
