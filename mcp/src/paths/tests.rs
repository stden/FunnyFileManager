//! Unit tests for root confinement.
//!
//! The fixtures shared by every group live here; the tests themselves are
//! grouped by the behaviour they pin in the child modules.

use super::ROOT_ENV;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tempfile::TempDir;

mod confinement;
mod normalization;
mod root_resolution;
#[cfg(unix)]
mod symlinks;

/// Roots are always canonical — `/tmp` itself is a symlink on some platforms.
fn root_of(dir: &TempDir) -> PathBuf {
    dir.path().canonicalize().expect("canonicalize temp dir")
}

/// `resolve_root` reads process-global state, so the tests that drive it take
/// turns on this lock.
static ROOT_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Holds the lock for one test and puts `$FFM_MCP_ROOT` and the working
/// directory back, so a later test never inherits the previous one's setup.
struct RootEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous_root: Option<OsString>,
    previous_cwd: PathBuf,
}

impl RootEnvGuard {
    fn acquire() -> Self {
        Self {
            // A test that panicked while holding the lock left the environment
            // restored by this guard's `Drop`, so the poison carries no risk.
            _lock: ROOT_ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner),
            previous_root: std::env::var_os(ROOT_ENV),
            previous_cwd: std::env::current_dir().expect("current directory"),
        }
    }
}

impl Drop for RootEnvGuard {
    fn drop(&mut self) {
        match &self.previous_root {
            Some(value) => std::env::set_var(ROOT_ENV, value),
            None => std::env::remove_var(ROOT_ENV),
        }
        if let Err(error) = std::env::set_current_dir(&self.previous_cwd) {
            eprintln!("could not restore the working directory: {error}");
        }
    }
}
