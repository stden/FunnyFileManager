//! Purpose: The AI clients FFM registers its MCP server with, in one table.
//!
//! The Install MCP server dialog, the installer and the `:mcp` screen all read
//! that table; the text-to-text logic that reads and edits the clients' config
//! files lives here too.
//!
//! Pure: nothing here touches the filesystem, starts a process or reads the
//! environment. `ops::mcp_install` does that, and reaches the machine only
//! through [`Host`], so tests hand it a fake one.

mod client;
mod codex_toml;
mod entry;
mod found;
mod host;
mod json;
mod state;
mod survey;
mod table;

pub use client::{Client, ConfigFormat, Mechanism, Scope};
pub use entry::{ProjectDir, ServerBinary, ServerEntry, ServerState};
pub use host::{Captured, CliCall, Exit, Host, Runner};
pub use state::{Blocker, ConfigState};
pub use survey::{row_label, server_lines, ClientStatus, Presence, Survey, INSTALL_HINT};
pub use table::{notes, ClientId, CLIENTS};

use found::{drops_launch_key, Args, FoundEntry, Launch, Switch, LAUNCH_KEYS};

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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_json;
#[cfg(test)]
mod tests_toml;
