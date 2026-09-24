//! Purpose: Say what FFM would start for a configured server, without starting
//! a process or installing a package.

use super::{environment::Environment, launch::Launch, probe::Probe, storage, InstallError};
use crate::config::{InstallRecipe, LspSettings, ServerSettings};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A program FFM would run, and the arguments it would pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

/// Why nothing is installed to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotInstalled {
    AutoInstallOff,
    NoCache,
    AfterTrust,
}

/// The configured command was not found, and the auto-install recipe's
/// program runs instead of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replacement {
    Cached(Resolved),
    AfterTrust,
}

/// How a status reads at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Pending,
    Bad,
}

/// What FFM would start for one server — never a process list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// The configured command was found.
    Configured(Resolved),
    /// The recipe that supplies the configured command is installed.
    Cached(Resolved),
    NotInstalled(NotInstalled),
    Replaced(Replacement),
    /// A corrupt receipt, an I/O error, or a missing runtime (Java 21, Node.js).
    Failed(InstallError),
}

impl Availability {
    /// The Status row's words.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Configured(_) => "found configured executable",
            Self::Cached(_) => "installed in FFM cache",
            Self::NotInstalled(NotInstalled::AfterTrust) => {
                "not installed; installs after workspace trust"
            }
            Self::NotInstalled(NotInstalled::AutoInstallOff) => {
                "not installed; auto-install disabled"
            }
            Self::NotInstalled(NotInstalled::NoCache) => "no installation cache configured",
            Self::Replaced(Replacement::Cached(_)) => {
                "command not found; FFM runs the auto-installed server instead"
            }
            Self::Replaced(Replacement::AfterTrust) => {
                "command not found; FFM will install and run the auto-install server instead"
            }
            Self::Failed(error) => error.label(),
        }
    }

    #[must_use]
    pub const fn tone(&self) -> Tone {
        match self {
            Self::Configured(_) | Self::Cached(_) => Tone::Good,
            Self::NotInstalled(NotInstalled::AfterTrust) | Self::Replaced(_) => Tone::Pending,
            Self::NotInstalled(_) | Self::Failed(_) => Tone::Bad,
        }
    }

    /// The one word the server list shows.
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Configured(_) | Self::Cached(_) => "ready",
            Self::NotInstalled(NotInstalled::AfterTrust) => "auto",
            Self::Replaced(_) => "recipe",
            Self::NotInstalled(_) => "missing",
            Self::Failed(_) => "error",
        }
    }

    /// The program FFM would run now, when it has one.
    #[must_use]
    pub const fn resolved(&self) -> Option<&Resolved> {
        match self {
            Self::Configured(resolved)
            | Self::Cached(resolved)
            | Self::Replaced(Replacement::Cached(resolved)) => Some(resolved),
            Self::NotInstalled(_) | Self::Replaced(Replacement::AfterTrust) | Self::Failed(_) => {
                None
            }
        }
    }
}

/// What FFM would start for `language`'s `server` in a file under
/// `directory`, on the machine `probe` describes.
#[must_use]
pub fn availability(
    language: &str,
    server: &ServerSettings,
    directory: &Path,
    probe: &Probe,
) -> Availability {
    let found = resolve(server, probe.cache(), &workspace(server, directory), probe);
    if recipe_supplies_command(language, server) {
        return found;
    }
    match found {
        Availability::Cached(resolved) => Availability::Replaced(Replacement::Cached(resolved)),
        Availability::NotInstalled(NotInstalled::AfterTrust) => {
            Availability::Replaced(Replacement::AfterTrust)
        }
        other => other,
    }
}

/// The project root the server would open for a file in `directory`.
fn workspace(server: &ServerSettings, directory: &Path) -> PathBuf {
    let directory = directory
        .canonicalize()
        .unwrap_or_else(|_| directory.to_path_buf());
    super::super::worker::workspace_root(&directory.join("source"), &server.root_markers)
}

/// Whether the auto-install recipe installs the command itself: the catalog's
/// command and recipe, both unchanged. Otherwise an installed recipe runs
/// *instead of* the configured command, and the status must say so.
fn recipe_supplies_command(language: &str, server: &ServerSettings) -> bool {
    LspSettings::catalog()
        .0
        .get(language)
        .is_some_and(|catalog| {
            catalog.command == server.command && catalog.install == server.install
        })
}

/// The configured command when it is found, else what the recipe offers.
fn resolve(
    server: &ServerSettings,
    cache: Option<&Path>,
    workspace: &Path,
    environment: &dyn Environment,
) -> Availability {
    if let Some(executable) = configured(&server.command, workspace, environment) {
        return Availability::Configured(Resolved {
            executable,
            args: server.args.clone(),
        });
    }
    let Some(recipe) = &server.install else {
        return Availability::NotInstalled(NotInstalled::AutoInstallOff);
    };
    let Some(cache) = cache else {
        return Availability::NotInstalled(NotInstalled::NoCache);
    };
    match cached(server, recipe, cache, workspace, environment) {
        Ok(Some(resolved)) => Availability::Cached(resolved),
        Ok(None) => Availability::NotInstalled(NotInstalled::AfterTrust),
        Err(error) => Availability::Failed(error),
    }
}

/// The recipe's installed program, `None` when it is not installed.
fn cached(
    server: &ServerSettings,
    recipe: &InstallRecipe,
    cache: &Path,
    workspace: &Path,
    environment: &dyn Environment,
) -> Result<Option<Resolved>, InstallError> {
    let key = storage::digest(
        &serde_json::to_vec(recipe).map_err(crate::lsp::logged(InstallError::Recipe))?,
    );
    let root = cache.join(key);
    if !root.exists() {
        return Ok(None);
    }
    let receipt: Launch = serde_json::from_slice(&fs::read(root.join("ready.json"))?)
        .map_err(crate::lsp::logged(InstallError::Recipe))?;
    let prepared = receipt.preview(server, &root, workspace, environment)?;
    Ok(Some(Resolved {
        executable: PathBuf::from(prepared.command),
        args: prepared.args,
    }))
}

fn configured(command: &str, workspace: &Path, environment: &dyn Environment) -> Option<PathBuf> {
    let path = Path::new(command);
    if path.is_absolute() {
        return super::launch::runnable(path).then(|| path.to_path_buf());
    }
    if path.components().count() != 1 {
        let path = workspace.join(path);
        return super::launch::runnable(&path).then_some(path);
    }
    environment.program(&[command], "not installed").ok()
}

#[cfg(test)]
mod tests;
