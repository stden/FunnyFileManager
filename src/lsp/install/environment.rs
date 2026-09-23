//! Purpose: Inject executable discovery and command execution at the installer boundary.

use super::{launch, runner, InstallError};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

pub(super) trait Environment {
    fn program(&self, names: &[&str], missing: &'static str) -> Result<PathBuf, InstallError>;
    fn run(&self, command: &mut Command, stop: &AtomicBool) -> Result<Vec<u8>, InstallError>;

    /// Every runnable match, `JAVA_HOME` first when `names` is `java`.
    /// The default is the single path [`Self::program`] would return.
    fn runtimes(&self, names: &[&str]) -> Vec<PathBuf> {
        self.program(names, "missing").into_iter().collect()
    }
}

pub(super) struct System;

impl Environment for System {
    fn program(&self, names: &[&str], missing: &'static str) -> Result<PathBuf, InstallError> {
        self.runtimes(names)
            .into_iter()
            .next()
            .ok_or(InstallError::Missing(missing))
    }

    fn run(&self, command: &mut Command, stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        runner::run(command, stop)
    }

    fn runtimes(&self, names: &[&str]) -> Vec<PathBuf> {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        let java_home = std::env::var_os("JAVA_HOME").map(PathBuf::from);
        candidates(names, std::env::split_paths(&paths), java_home.as_deref())
    }
}

#[cfg(test)]
fn discover(
    names: &[&str],
    missing: &'static str,
    paths: impl Iterator<Item = PathBuf>,
    java_home: Option<&Path>,
) -> Result<PathBuf, InstallError> {
    candidates(names, paths, java_home)
        .into_iter()
        .next()
        .ok_or(InstallError::Missing(missing))
}

pub(super) fn candidates(
    names: &[&str],
    paths: impl Iterator<Item = PathBuf>,
    java_home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if names == ["java"] {
        if let Some(java) = java_home.and_then(home_java) {
            found.push(java);
        }
    }
    for path in launch::find_all(names, paths) {
        if !found.iter().any(|existing| existing == &path) {
            found.push(path);
        }
    }
    found
}

fn home_java(root: &Path) -> Option<PathBuf> {
    if !root.is_absolute() {
        return None;
    }
    let java = root.join("bin").join(launch::executable("java"));
    launch::runnable(&java).then_some(java)
}

#[cfg(test)]
mod tests;
