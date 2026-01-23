use std::ffi::OsString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn editor_env_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn editor_env_lock() -> MutexGuard<'static, ()> {
    editor_env_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) struct EditorEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

impl EditorEnvGuard {
    pub(crate) fn set(path: &Path) -> Self {
        let lock = editor_env_lock();
        Self::set_with_lock(lock, path)
    }

    pub(crate) fn clear() -> Self {
        let lock = editor_env_lock();
        Self::clear_with_lock(lock)
    }

    pub(crate) fn set_with_lock(lock: MutexGuard<'static, ()>, path: &Path) -> Self {
        let previous = std::env::var_os("EDITOR");
        std::env::set_var("EDITOR", path);
        Self {
            _lock: lock,
            previous,
        }
    }

    pub(crate) fn clear_with_lock(lock: MutexGuard<'static, ()>) -> Self {
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

pub(crate) fn restore_editor_env(value: Option<OsString>) {
    match value {
        Some(value) => std::env::set_var("EDITOR", value),
        None => std::env::remove_var("EDITOR"),
    }
}
