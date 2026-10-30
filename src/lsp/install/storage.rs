//! Purpose: Serialize concurrent installs and publish complete server directories atomically.

use super::{
    check_cancel,
    environment::{Environment, System},
    launch::Launch,
    recipes, InstallError,
};
use crate::config::ServerSettings;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, TryLockError},
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(super) fn prepare(
    settings: &ServerSettings,
    cache: &Path,
    workspace: &Path,
    stop: &AtomicBool,
) -> Result<ServerSettings, InstallError> {
    prepare_with(settings, cache, workspace, stop, &System)
}

pub(super) fn prepare_with(
    settings: &ServerSettings,
    cache: &Path,
    workspace: &Path,
    stop: &AtomicBool,
    environment: &dyn Environment,
) -> Result<ServerSettings, InstallError> {
    check_cancel(stop)?;
    let recipe = settings.install.as_ref().ok_or(InstallError::Recipe)?;
    let encoded = serde_json::to_vec(recipe).map_err(crate::lsp::logged(InstallError::Recipe))?;
    let key = digest(&encoded);
    crate::file_write::create_dirs_owner_only(cache).map_err(InstallError::cache)?;
    let _lock = lock(&cache.join(format!("{key}.lock")), stop)?;
    let destination = cache.join(&key);
    if !destination.exists() {
        let staging = tempfile::Builder::new()
            .prefix("install-")
            .tempdir_in(cache)
            .map_err(InstallError::cache)?;
        recipes::install(recipe, staging.path(), stop, environment)?;
        let launch = super::launch::for_recipe(recipe, staging.path(), stop, environment)?;
        let receipt =
            serde_json::to_vec(&launch).map_err(crate::lsp::logged(InstallError::Recipe))?;
        fs::write(staging.path().join("ready.json"), receipt)?;
        check_cancel(stop)?;
        fs::rename(staging.path(), &destination)?;
    }
    let receipt = read_receipt(&destination)?;
    let launch: Launch =
        serde_json::from_slice(&receipt).map_err(crate::lsp::logged(InstallError::Recipe))?;
    launch.settings(settings, &destination, workspace, environment, stop)
}

fn lock(path: &Path, stop: &AtomicBool) -> Result<File, InstallError> {
    lock_until(
        path,
        stop,
        Instant::now() + Duration::from_secs(600),
        || {
            std::thread::sleep(Duration::from_millis(20));
        },
    )
}

fn lock_until(
    path: &Path,
    stop: &AtomicBool,
    deadline: Instant,
    mut wait: impl FnMut(),
) -> Result<File, InstallError> {
    // A FIFO in the lock's place is refused, never waited on.
    let lock = crate::fileload::open_regular(
        path,
        File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false),
    )
    .map_err(InstallError::cache)?;
    loop {
        check_cancel(stop)?;
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                wait();
            }
            Err(TryLockError::WouldBlock) => return Err(InstallError::Timeout),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

/// The `ready.json` receipt of the installation in `root`, read with the
/// guards of every data file FFM keeps.
pub(super) fn read_receipt(root: &Path) -> std::io::Result<Vec<u8>> {
    crate::fileload::read_small_file(&root.join("ready.json"), "ready.json").map(String::into_bytes)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 15])
        // A nibble is always a hexadecimal digit, lowercase from `from_digit`.
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

pub(super) fn relative(root: &Path, name: &str) -> Result<std::path::PathBuf, InstallError> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains(['\\', ':'])
        || path.components().any(|part| {
            !matches!(
                part,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(InstallError::Recipe);
    }
    Ok(root.join(path))
}

#[cfg(test)]
mod tests;
