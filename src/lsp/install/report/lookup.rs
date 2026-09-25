//! Purpose: Find what FFM would start for a server — the configured command
//! when it is found, else the auto-install recipe's installed program.

use std::path::{Path, PathBuf};

use super::{Availability, NotInstalled, Replacement, Resolved};
use crate::config::{InstallRecipe, LspSettings, ServerSettings};
use crate::lsp::install::{
    environment::Environment, launch::Launch, probe::Probe, recipes, storage, InstallError,
};

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
pub(super) fn workspace(server: &ServerSettings, directory: &Path) -> PathBuf {
    let directory = directory
        .canonicalize()
        .unwrap_or_else(|_| directory.to_path_buf());
    crate::lsp::worker::workspace_root(&directory.join("source"), &server.root_markers)
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
pub(super) fn resolve(
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
        // Installs after trust only if what it installs with is here.
        Ok(None) => recipes::runtime(recipe, environment).map_or_else(Availability::Failed, |()| {
            Availability::NotInstalled(NotInstalled::AfterTrust)
        }),
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
    let receipt: Launch = serde_json::from_slice(&storage::read_receipt(&root)?)
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
        return crate::lsp::install::launch::runnable(path).then(|| path.to_path_buf());
    }
    if path.components().count() != 1 {
        let path = workspace.join(path);
        return crate::lsp::install::launch::runnable(&path).then_some(path);
    }
    environment.program(&[command], "not installed").ok()
}
