//! Purpose: Where each client keeps its config — its home folder, its user
//! config (Claude Code's chosen as its CLI chooses) and a folder's project
//! config — and whether two of those are one file.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::Target;
use crate::mcp_clients::{Blocker, Client, ClientId, Host, Scope, CLAUDE_CUSTOM_OAUTH_VAR};

/// Claude Code's own folder under the home when `CLAUDE_CONFIG_DIR` is unset.
const CLAUDE_DIR: &str = ".claude";

/// Claude Code's legacy user config, in its own folder: the CLI reads and
/// writes it in place of `.claude.json` whenever it exists.
const CLAUDE_LEGACY_CONFIG: &str = ".config.json";

/// Claude Code's user config while `CLAUDE_CODE_CUSTOM_OAUTH_URL` is set.
const CLAUDE_CUSTOM_OAUTH_CONFIG: &str = ".claude-custom-oauth.json";

/// `relative`, written with `/` in the table, joined one part at a time so
/// Windows shows `\`.
#[must_use]
pub fn join_relative(base: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(base.to_path_buf(), |path, part| path.join(part))
}

/// The client's home folder: its override variable when set, else its folder
/// under the user's home. Either must be a folder that exists: registering
/// never creates a home.
pub fn client_home(host: &Host, client: &Client) -> Result<PathBuf, Blocker> {
    if let Some((var, dir)) = client
        .home_var
        .and_then(|var| host.env.get(var).map(|dir| (var, PathBuf::from(dir))))
    {
        // Codex refuses to start when `CODEX_HOME` is not a folder; nothing
        // is guessed for the others either.
        return if dir.is_dir() {
            Ok(dir)
        } else {
            Err(Blocker::HomeVarNotFolder(var))
        };
    }
    host.home
        .as_deref()
        .filter(|home| home.is_dir())
        .map(|home| join_relative(home, client.home_dir))
        .ok_or(Blocker::NoHomeFolder)
}

/// The client's user-level config, where the client itself looks for it.
pub fn user_config(host: &Host, client: &Client) -> Result<PathBuf, Blocker> {
    let home = client_home(host, client)?;
    Ok(match client.id {
        ClientId::ClaudeCode => claude_user_config(host, &home),
        ClientId::Codex | ClientId::Junie | ClientId::Kiro => {
            join_relative(&home, client.user_config)
        }
    })
}

/// Claude Code's user config, chosen as its CLI chooses: a legacy
/// `.config.json` in its own folder (`$CLAUDE_CONFIG_DIR`, else `~/.claude`)
/// wins whenever it exists; otherwise `.claude.json` in `home` — the
/// variable's folder, else the home — or `.claude-custom-oauth.json` while a
/// custom OAuth server is set.
fn claude_user_config(host: &Host, home: &Path) -> PathBuf {
    let client = ClientId::ClaudeCode.client();
    let own_dir = match client.home_var.filter(|var| host.env.contains_key(var)) {
        Some(_) => home.to_path_buf(),
        None => home.join(CLAUDE_DIR),
    };
    let legacy = own_dir.join(CLAUDE_LEGACY_CONFIG);
    if legacy.exists() {
        return legacy;
    }
    if host.env.contains_key(CLAUDE_CUSTOM_OAUTH_VAR) {
        home.join(CLAUDE_CUSTOM_OAUTH_CONFIG)
    } else {
        join_relative(home, client.user_config)
    }
}

/// The client's config file for `target`.
pub fn config_path(host: &Host, client: &Client, target: Target<'_>) -> Result<PathBuf, Blocker> {
    match target {
        Target::CurrentFolder(folder) => Ok(join_relative(folder.as_path(), client.project_config)),
        Target::AllProjects => user_config(host, client),
    }
}

/// [`Blocker::SameAsUserConfig`] when `path`, the client's config for a
/// folder, is its user-level config under another spelling: a Current folder
/// entry written there would confine every project to that folder.
pub fn shared_with_all_projects(
    host: &Host,
    client: &Client,
    target: Target<'_>,
    path: &Path,
) -> Option<Blocker> {
    if target.scope() != Scope::CurrentFolder {
        return None;
    }
    let user = user_config(host, client).ok()?;
    (resolved(&user) == resolved(path)).then_some(Blocker::SameAsUserConfig)
}

/// `path` with its longest existing ancestor resolved — links, `..`, and on
/// Windows the spelling — and the rest, which does not exist yet, appended:
/// two spellings of one file, created or not, come out equal.
fn resolved(path: &Path) -> PathBuf {
    path.ancestors()
        .find_map(|ancestor| {
            let rest = path.strip_prefix(ancestor).ok()?;
            fs::canonicalize(ancestor).ok().map(|real| real.join(rest))
        })
        .unwrap_or_else(|| path.to_path_buf())
}
