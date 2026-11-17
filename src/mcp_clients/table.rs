//! Purpose: The four clients FFM registers with, as rows of one table, and
//! the `ClientId` that names each row.

use super::{Client, ConfigFormat, Mechanism, Scope};

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

/// Every client's note for `scope`, in table order: what the dialog shows
/// under the rows, the same whichever rows are checked.
#[must_use]
pub fn notes(scope: Scope) -> Vec<String> {
    CLIENTS
        .iter()
        .filter_map(|client| client.note_line(scope))
        .collect()
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
