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
    let encoded = serde_json::to_vec(recipe).map_err(|_| InstallError::Recipe)?;
    let key = digest(&encoded);
    fs::create_dir_all(cache)?;
    let _lock = lock(&cache.join(format!("{key}.lock")), stop)?;
    let destination = cache.join(&key);
    if !destination.exists() {
        let staging = tempfile::Builder::new()
            .prefix("install-")
            .tempdir_in(cache)?;
        recipes::install(recipe, staging.path(), stop, environment)?;
        let launch = super::launch::for_recipe(recipe, staging.path(), stop, environment)?;
        let receipt = serde_json::to_vec(&launch).map_err(|_| InstallError::Recipe)?;
        fs::write(staging.path().join("ready.json"), receipt)?;
        check_cancel(stop)?;
        fs::rename(staging.path(), &destination)?;
    }
    let receipt = fs::read(destination.join("ready.json"))?;
    let launch: Launch = serde_json::from_slice(&receipt).map_err(|_| InstallError::Recipe)?;
    launch.settings(settings, &destination, workspace)
}

fn lock(path: &Path, stop: &AtomicBool) -> Result<File, InstallError> {
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        check_cancel(stop)?;
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(TryLockError::WouldBlock) => return Err(InstallError::Timeout),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

pub(super) fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ]
        })
        .map(char::from)
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
