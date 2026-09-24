//! Purpose: Look at the machine for the Install MCP server dialog and the
//! `:mcp` screen — where the server is, which clients are installed, and what
//! each config says. The only place the installer reads the filesystem.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::fileload::{self, Loaded};
use crate::mcp_clients::{
    self, Blocker, Client, ClientId, ClientStatus, ConfigFormat, ConfigState, Host, Mechanism,
    Presence, ProjectDir, Scope, ServerBinary, ServerEntry, ServerState, Survey,
    CLAUDE_CUSTOM_OAUTH_VAR, CLIENTS, SERVER_NAME,
};

/// The most of a config that is read. Claude Code's `.claude.json` keeps a
/// history for every project and runs to megabytes; a config past this is
/// not one to edit blind.
const MAX_CONFIG_BYTES: u64 = 64 * 1024 * 1024;

/// Claude Code's own folder under the home when `CLAUDE_CONFIG_DIR` is unset.
const CLAUDE_DIR: &str = ".claude";

/// Claude Code's legacy user config, in its own folder: the CLI reads and
/// writes it in place of `.claude.json` whenever it exists.
const CLAUDE_LEGACY_CONFIG: &str = ".config.json";

/// Claude Code's user config while `CLAUDE_CODE_CUSTOM_OAUTH_URL` is set.
const CLAUDE_CUSTOM_OAUTH_CONFIG: &str = ".claude-custom-oauth.json";

/// Where a registration goes: a scope, with the folder it needs.
///
/// A scope and an optional folder would let "current folder" exist without a
/// folder; this cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    AllProjects,
    CurrentFolder(&'a ProjectDir),
}

impl<'a> Target<'a> {
    /// `scope`, or all projects when there is no folder for the current one.
    #[must_use]
    pub const fn new(scope: Scope, folder: Option<&'a ProjectDir>) -> Self {
        match (scope, folder) {
            (Scope::CurrentFolder, Some(folder)) => Self::CurrentFolder(folder),
            _ => Self::AllProjects,
        }
    }

    #[must_use]
    pub const fn scope(self) -> Scope {
        match self {
            Self::AllProjects => Scope::AllProjects,
            Self::CurrentFolder(_) => Scope::CurrentFolder,
        }
    }

    /// The folder the server is confined to, which only a folder has.
    #[must_use]
    pub const fn root(self) -> Option<&'a ProjectDir> {
        match self {
            Self::AllProjects => None,
            Self::CurrentFolder(folder) => Some(folder),
        }
    }

    /// The entry a registration here writes for `server`.
    #[must_use]
    pub fn entry(self, server: &ServerBinary) -> ServerEntry {
        ServerEntry {
            command: server.clone(),
            root: self.root().cloned(),
        }
    }
}

/// The first `name` in a PATH folder that can be run, as an absolute path.
/// Relative PATH entries are skipped: they name wherever FFM happens to be.
#[must_use]
pub fn find_program(host: &Host, name: &str) -> Option<PathBuf> {
    host.search_path
        .iter()
        .filter(|dir| dir.is_absolute())
        .find_map(|dir| executable_in(host, dir, name))
}

/// The first program the client table lists for `client` that is on PATH.
#[must_use]
pub fn client_program(host: &Host, client: &Client) -> Option<PathBuf> {
    client
        .programs
        .iter()
        .find_map(|program| find_program(host, program))
}

fn executable_in(host: &Host, dir: &Path, name: &str) -> Option<PathBuf> {
    host.program_suffixes
        .iter()
        .map(|suffix| dir.join(format!("{name}{suffix}")))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    /// Any of the owner, group and other execute bits.
    const EXECUTE: u32 = 0o111;
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & EXECUTE != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// `ffm-mcp` next to the running `ffm` — where `cargo install --path mcp`
/// leaves it — else on PATH. The path is used as found: canonicalizing it
/// would put a `\\?\` prefix into a Windows config.
#[must_use]
pub fn locate_server(host: &Host) -> ServerState {
    let beside_ffm = host
        .exe_dir
        .as_deref()
        .and_then(|dir| executable_in(host, dir, SERVER_NAME));
    match beside_ffm.or_else(|| find_program(host, SERVER_NAME)) {
        None => ServerState::Missing,
        Some(path) => {
            ServerBinary::new(path).map_or_else(ServerState::NotUnicode, ServerState::Found)
        }
    }
}

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

/// Installed, as far as can be told: on PATH, a user config, or a home
/// folder of its own.
fn presence(host: &Host, client: &Client) -> Presence {
    let has_config = user_config(host, client).is_ok_and(|config| config.is_file());
    let has_home =
        !client.home_dir.is_empty() && client_home(host, client).is_ok_and(|home| home.is_dir());
    if has_config || has_home || client_program(host, client).is_some() {
        Presence::Detected
    } else {
        Presence::NotDetected
    }
}

/// A config's text, `None` when there is no file — including when a folder
/// on the way to it is a file, which the write then reports.
///
/// Only a regular file of at most [`MAX_CONFIG_BYTES`] is read: opening a
/// FIFO named `.mcp.json` would block the dialog forever, and a link to
/// `/dev/zero` would fill the memory (see [`fileload::read_regular`]).
pub fn load(path: &Path) -> Result<Option<String>, Blocker> {
    match fileload::read_regular(path, MAX_CONFIG_BYTES) {
        Ok(Loaded::Bytes(bytes)) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| Blocker::Unreadable(format!("not UTF-8 text ({error})"))),
        Ok(Loaded::NotRegular) => Err(Blocker::Unreadable("not a regular file".to_owned())),
        Ok(Loaded::TooLarge(size)) => Err(Blocker::Unreadable(format!(
            "{size} bytes, more than the {MAX_CONFIG_BYTES} a config may have"
        ))),
        Err(error) => match error.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Ok(None),
            _ => Err(Blocker::Unreadable(error.to_string())),
        },
    }
}

/// What the config at `path` says about `desired`.
#[must_use]
pub fn read_state(path: &Path, format: ConfigFormat, desired: Option<&ServerEntry>) -> ConfigState {
    match load(path) {
        Ok(None) => ConfigState::NoConfig,
        Ok(Some(text)) => check_program(mcp_clients::state(format, &text, desired)),
        Err(blocker) => ConfigState::Blocked(blocker),
    }
}

/// An entry nothing could be compared with still names what it runs: an
/// absolute path with nothing behind it is an entry that cannot start. A bare
/// name is the client's PATH's business, which FFM cannot see.
fn check_program(state: ConfigState) -> ConfigState {
    match state {
        ConfigState::Unverified {
            command: Some(command),
        } if Path::new(&command).is_absolute() && !Path::new(&command).exists() => {
            ConfigState::ProgramMissing
        }
        other => other,
    }
}

fn client_status(
    host: &Host,
    client: &Client,
    target: Target<'_>,
    desired: Option<&ServerEntry>,
) -> ClientStatus {
    let (config, state) = match config_path(host, client, target) {
        Ok(path) => {
            let state = shared_with_all_projects(host, client, target, &path).map_or_else(
                || read_state(&path, client.format, desired),
                ConfigState::Blocked,
            );
            (Some(path), state)
        }
        Err(blocker) => (None, ConfigState::Blocked(blocker)),
    };
    // Only `claude` writes Claude Code's user config: without it on PATH a
    // change cannot be made, though a registration already there still is.
    let needs_cli = client.mechanism(target.scope()) == Mechanism::ClaudeCli
        && state.is_actionable()
        && client_program(host, client).is_none();
    ClientStatus {
        client: client.id,
        presence: presence(host, client),
        config,
        state: if needs_cli {
            ConfigState::Blocked(Blocker::NeedsClaudeCli)
        } else {
            state
        },
    }
}

/// Every client's state at `scope`, the server, and the folder.
#[must_use]
pub fn survey(host: &Host, scope: Scope, folder: Option<ProjectDir>) -> Survey {
    let server = locate_server(host);
    let target = Target::new(scope, folder.as_ref());
    let desired = server.binary().map(|binary| target.entry(binary));
    let clients = CLIENTS
        .iter()
        .map(|client| client_status(host, client, target, desired.as_ref()))
        .collect();
    Survey {
        scope: target.scope(),
        server,
        folder,
        clients,
    }
}

/// The active panel's folder as a project, when it is a local folder a config
/// can name: an S3 listing or a folder that has gone is not.
#[must_use]
pub fn project_dir(path: &Path) -> Option<ProjectDir> {
    if path.is_dir() {
        ProjectDir::new(path.to_path_buf())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_paths;
