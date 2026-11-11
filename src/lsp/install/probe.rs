//! Purpose: Everything the Language servers dialog asks of this machine — the
//! LSP install cache, the `PATH` and `JAVA_HOME` used to find programs, and the
//! program names offered for Command — held in one value on `App`.
//!
//! One value so that ops tests and the TUI BDD can hand the dialog a fake
//! machine: a `TempDir` cache and a scanned fake `PATH`. No test then reads the
//! real `~/.config/ffm/lsp` or the real `PATH`.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

use super::{environment::Environment, InstallError};
use crate::complete::execs;

/// Where the program names offered for Command come from.
#[derive(Debug, Clone)]
pub enum Programs {
    /// A list scanned once, from the `PATH` a test gave.
    Listed(Vec<String>),
    /// The process `PATH`, scanned on a background thread.
    Process,
}

impl Default for Programs {
    fn default() -> Self {
        Self::Listed(Vec::new())
    }
}

/// What the Language servers dialog asks of this machine (see the module).
/// The default knows nothing: no cache, no `PATH`, no programs.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    cache: Option<PathBuf>,
    path: Option<OsString>,
    java_home: Option<PathBuf>,
    programs: Programs,
}

impl Probe {
    /// This process's machine: the real cache, `PATH` and `JAVA_HOME`.
    #[must_use]
    pub fn from_process() -> Self {
        Self {
            cache: super::cache_dir(),
            path: std::env::var_os("PATH"),
            java_home: std::env::var_os("JAVA_HOME").map(PathBuf::from),
            programs: Programs::Process,
        }
    }

    /// A machine with `cache` and `path` and nothing else: no `JAVA_HOME`, and
    /// the programs of `path`, scanned now. A seam for tests: production
    /// probes the process.
    #[cfg(any(test, feature = "test-helpers"))]
    #[must_use]
    pub fn new(cache: Option<PathBuf>, path: Option<OsString>) -> Self {
        let programs = Programs::Listed(execs::scan(path.as_deref()));
        Self {
            cache,
            path,
            java_home: None,
            programs,
        }
    }

    /// The LSP install cache, when there is one.
    #[must_use]
    pub fn cache(&self) -> Option<&Path> {
        self.cache.as_deref()
    }

    /// The program names offered for Command, sorted. Never blocks: the
    /// process's `PATH` is empty here until its background scan finishes.
    #[must_use]
    pub fn programs(&self) -> &[String] {
        match &self.programs {
            Programs::Listed(names) => names,
            Programs::Process => execs::ready().unwrap_or_default(),
        }
    }

    /// Start scanning the process `PATH`, once; a listed probe has nothing to do.
    pub fn warm(&self) {
        if matches!(self.programs, Programs::Process) {
            execs::warm();
        }
    }
}

/// The same lookup rule as the runtime's [`super::environment::System`], over
/// this probe's `PATH` and `JAVA_HOME`.
impl Environment for Probe {
    fn program(&self, names: &[&str], missing: &'static str) -> Result<PathBuf, InstallError> {
        self.runtimes(names)
            .into_iter()
            .next()
            .ok_or(InstallError::Missing(missing))
    }

    /// Availability only inspects; it never runs a program, so this is never
    /// reached. Refusing keeps a mistake from starting one.
    fn run(&self, _command: &mut Command, _stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        Err(InstallError::Command)
    }

    fn runtimes(&self, names: &[&str]) -> Vec<PathBuf> {
        let path = self.path.clone().unwrap_or_default();
        super::environment::candidates(
            names,
            std::env::split_paths(&path),
            self.java_home.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests;
