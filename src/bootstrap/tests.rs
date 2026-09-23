//! Unit tests: start-up loads what earlier runs stored — here the `NetBox`
//! sessions — from sandboxed configuration and data directories, never the
//! user's own; and the line a failed session leaves on stderr at exit.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::MutexGuard;

use tempfile::TempDir;

use super::{load_persisted, AtExitErrorLine};
use crate::sessions::{SessionStore, FILE_NAME};
use crate::test_support::netbox::{app_in, file_store, name, session};

/// Every variable the stores read their directories from.
const HOMES: [&str; 4] = ["HOME", "APPDATA", "XDG_CONFIG_HOME", "XDG_DATA_HOME"];

/// All of [`HOMES`] pointed into one temporary directory for as long as it
/// lives, under the crate-wide environment lock, and restored on drop.
struct SandboxedHomes {
    _lock: MutexGuard<'static, ()>,
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl SandboxedHomes {
    fn new(root: &Path) -> Self {
        let lock = crate::test_support::env_lock();
        let previous = HOMES
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in HOMES {
            std::env::set_var(name, root.join(name.to_lowercase()));
        }
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for SandboxedHomes {
    fn drop(&mut self) {
        for (name, value) in self.previous.drain(..) {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[test]
fn start_up_loads_the_stored_sessions() {
    let root = TempDir::new().expect("temp dir");
    let _homes = SandboxedHomes::new(root.path());
    let config = SessionStore::path().expect("a sandboxed config directory");
    let dir = config.parent().expect("the ffm directory");
    fs::create_dir_all(dir).expect("mkdir");
    drop(file_store(dir, vec![session("a.test", None)]));
    assert!(dir.join(FILE_NAME).is_file());

    let panels = TempDir::new().expect("temp dir");
    let mut app = app_in(panels.path());
    load_persisted(&mut app);
    assert!(app.sessions.find(&name("a.test")).is_some());
    assert!(!app.sessions.is_damaged());
}

#[test]
fn a_custom_error_is_reported_with_its_kind_and_message() {
    let error = io::Error::other("terminal gone");
    assert_eq!(
        AtExitErrorLine(&error).to_string(),
        r#"Error: Custom { kind: Other, error: "terminal gone" }"#
    );
}

#[test]
fn a_bare_kind_is_reported_by_name() {
    let error = io::Error::from(io::ErrorKind::BrokenPipe);
    assert_eq!(
        AtExitErrorLine(&error).to_string(),
        "Error: Kind(BrokenPipe)"
    );
}
