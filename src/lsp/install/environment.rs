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
}

pub(super) struct System;

impl Environment for System {
    fn program(&self, names: &[&str], missing: &'static str) -> Result<PathBuf, InstallError> {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        let java_home = std::env::var_os("JAVA_HOME").map(PathBuf::from);
        discover(
            names,
            missing,
            std::env::split_paths(&paths),
            java_home.as_deref(),
        )
    }

    fn run(&self, command: &mut Command, stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        runner::run(command, stop)
    }
}

fn discover(
    names: &[&str],
    missing: &'static str,
    paths: impl Iterator<Item = PathBuf>,
    java_home: Option<&Path>,
) -> Result<PathBuf, InstallError> {
    if names == ["java"] {
        if let Some(root) = java_home.filter(|root| root.is_absolute()) {
            let java = root.join("bin").join(launch::executable("java"));
            if launch::runnable(&java) {
                return Ok(java);
            }
        }
    }
    launch::find_any(names, paths, missing)
}

#[cfg(test)]
mod tests;
