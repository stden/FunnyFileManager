//! Purpose: What the installer needs from the machine — home folder, PATH,
//! the executable's folder and a way to run a client CLI — as one value that
//! production fills from the real machine and tests fill from temp folders.

use std::{collections::BTreeMap, ffi::OsString, io, path::PathBuf, sync::Arc, time::Duration};

/// The machine as the installer sees it.
///
/// `Default` is inert: no home, an empty PATH, no executable folder and a
/// runner that runs nothing. Every `App` starts with it, so no test can reach
/// the real home folder or start a real client by accident; only the
/// production entry point installs the real machine.
#[derive(Clone)]
pub struct Host {
    pub home: Option<PathBuf>,
    /// Only the variables the installer reads ([`super::env_vars`]), by
    /// name, when set to something.
    pub env: BTreeMap<&'static str, OsString>,
    pub search_path: Vec<PathBuf>,
    /// What a program's name may end in: `[""]` on Unix, PATHEXT on Windows.
    pub program_suffixes: Vec<String>,
    /// The folder the running `ffm` lives in.
    pub exe_dir: Option<PathBuf>,
    pub runner: Arc<dyn Runner>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            home: None,
            env: BTreeMap::new(),
            search_path: Vec::new(),
            program_suffixes: Vec::new(),
            exe_dir: None,
            runner: Arc::new(NoRunner),
        }
    }
}

/// Runs one client CLI to completion.
pub trait Runner: Send + Sync {
    fn run(&self, call: &CliCall) -> io::Result<Captured>;
}

/// One non-interactive run of a client CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCall {
    /// Always an absolute path: `Command` does not apply PATHEXT to a bare name.
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub timeout: Duration,
}

/// What a finished run printed, and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Success,
    /// A non-zero code, or `None` when a signal ended it.
    Failure(Option<i32>),
}

/// The default host's runner: it runs nothing.
struct NoRunner;

impl Runner for NoRunner {
    fn run(&self, _call: &CliCall) -> io::Result<Captured> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no program runner is installed",
        ))
    }
}
