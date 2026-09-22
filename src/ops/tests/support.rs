//! Shared fixtures for operation regression tests.

use crate::ops::file::{FORCE_RENAME_FAILURE, RENAME_TEST_MUTEX};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::atomic::Ordering;

#[cfg(feature = "test-helpers")]
pub(super) fn fake_exit_status(code: i32) -> std::process::ExitStatus {
    use std::process::Command;
    if code == 0 {
        Command::new("true").status().unwrap()
    } else {
        Command::new("false").status().unwrap()
    }
}

pub(super) struct ForceRenameFailureGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl ForceRenameFailureGuard {
    pub(crate) fn enable() -> Self {
        let lock = RENAME_TEST_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
        Self { _lock: lock }
    }
}

impl Drop for ForceRenameFailureGuard {
    fn drop(&mut self) {
        FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    }
}

pub(super) fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

pub(super) fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}
