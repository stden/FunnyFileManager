//! Purpose: Look at the machine for the Install MCP server dialog and the
//! `:mcp` screen — where the server is, which clients are installed, and what
//! each config says. The only place the installer reads the filesystem.

use std::path::Path;

use crate::mcp_clients::{
    Blocker, Client, ClientStatus, ConfigState, Host, Mechanism, Presence, ProjectDir, Scope,
    ServerBinary, ServerEntry, Survey, CLIENTS,
};

mod config_paths;
mod config_read;
mod programs;

pub use config_paths::{client_home, config_path, shared_with_all_projects, user_config};
pub use config_read::{load, read_state};
pub use programs::{client_program, locate_server};

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
