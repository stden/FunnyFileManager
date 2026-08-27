//! Test utilities for process-global environment management.
//! Provides a single crate-wide lock for any `set_var` / `remove_var` in tests.

#![allow(dead_code)]

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Process-wide mutex for **any** mutation of process environment variables in
/// tests (`EDITOR`, `HOME`, `USERPROFILE`, `XDG_DATA_HOME`, …).
///
/// Every test that calls `std::env::set_var` / `remove_var` must hold this lock
/// for the whole mutation window. Split per-variable mutexes race because the
/// environment is one shared map.
fn env_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Acquire the crate-wide environment lock (poison-tolerant).
pub fn env_lock() -> MutexGuard<'static, ()> {
    env_mutex().lock().unwrap_or_else(PoisonError::into_inner)
}

/// Alias kept for existing call sites that only touch `$EDITOR`.
pub fn editor_env_lock() -> MutexGuard<'static, ()> {
    env_lock()
}

pub struct EditorEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

impl EditorEnvGuard {
    pub fn set(path: &Path) -> Self {
        let lock = env_lock();
        Self::set_with_lock(lock, path)
    }

    pub fn clear() -> Self {
        let lock = env_lock();
        Self::clear_with_lock(lock)
    }

    pub fn set_with_lock(lock: MutexGuard<'static, ()>, path: &Path) -> Self {
        let previous = std::env::var_os("EDITOR");
        std::env::set_var("EDITOR", path);
        Self {
            _lock: lock,
            previous,
        }
    }

    pub fn clear_with_lock(lock: MutexGuard<'static, ()>) -> Self {
        let previous = std::env::var_os("EDITOR");
        std::env::remove_var("EDITOR");
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for EditorEnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::env::set_var("EDITOR", previous);
        } else {
            std::env::remove_var("EDITOR");
        }
    }
}

pub fn restore_editor_env(value: Option<OsString>) {
    match value {
        Some(value) => std::env::set_var("EDITOR", value),
        None => std::env::remove_var("EDITOR"),
    }
}

/// Create a named pipe at `path` with `mkfifo(1)`.
///
/// A FIFO with no writer is the nastiest input the F3/F4 guards face: opening
/// it blocks in the kernel and never returns. Tests need a real one — a plain
/// file cannot reproduce it.
#[cfg(unix)]
pub fn make_fifo(path: &Path) -> std::io::Result<()> {
    let status = std::process::Command::new("mkfifo").arg(path).status()?;
    if status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(format!(
        "mkfifo {} failed with {status}",
        path.display()
    )))
}

/// Run `body` on a scratch thread and give up after `timeout`.
///
/// Returns `None` when the deadline passes, so a handler that blocks forever
/// *fails* the suite instead of hanging it — the whole point of the FIFO tests.
/// The stuck thread is deliberately left detached: the test harness's process
/// exits when the main thread finishes and takes it along.
pub fn run_with_timeout<T, F>(timeout: std::time::Duration, body: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver is gone on timeout; that send error is expected.
        drop(sender.send(body()));
    });
    receiver.recv_timeout(timeout).ok()
}
