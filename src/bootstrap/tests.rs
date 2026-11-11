//! Unit tests: start-up loads what earlier runs stored — here the `NetBox`
//! sessions — from sandboxed configuration and data directories, never the
//! user's own; and the line a failed session leaves on stderr at exit.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::MutexGuard;

use tempfile::TempDir;

use super::at_exit::AtExitErrorLine;
use super::load_persisted;
use crate::sessions::{SessionStore, FILE_NAME};
use crate::test_support::netbox::{app_in, file_store, name, session};

#[cfg(unix)]
mod abort_hook;
mod startup;

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
    let missing = Err(io::Error::from(io::ErrorKind::NotFound));
    let session_only = crate::history::Persistence::SessionOnly;
    assert!(super::load_app_from(missing, None, None, session_only).is_err());
}

#[test]
fn load_app_install_hosts_and_panic_hook_work_in_sandboxed_home() {
    let root = TempDir::new().expect("temp dir");
    let _homes = SandboxedHomes::new(root.path());
    let mut app = super::load_app().expect("load_app");
    super::install_hosts(&mut app, &crate::recording::Input::Live(None));
    assert!(app.live.elevation);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while crate::complete::execs::ready().is_none() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(crate::complete::execs::ready().is_some());

    let rec_path = root.path().join("events.jsonl");
    fs::write(
        &rec_path,
        r#"{"version":1,"executable":"ffm","directory":".","size":[80,24]}"#,
    )
    .expect("write header");
    let (replay, _) =
        crate::recording::Input::from_args(&["--replay".into(), rec_path.into()], (80, 24))
            .expect("replay args");
    super::install_hosts(&mut app, &replay);
    assert!(!app.live.elevation);

    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let saved = std::panic::take_hook();
    let noop = |_: &std::panic::PanicHookInfo<'_>| {};
    std::panic::set_hook(super::panic_restore_hook(Box::new(noop)));
    let caught = std::panic::catch_unwind(|| panic!("abort hook exercise"));
    std::panic::set_hook(saved);
    assert!(caught.is_err());
}

#[test]
fn end_session_logs_persistence_failures_and_suppresses_lost_terminal_errors() {
    let root = TempDir::new().expect("temp dir");
    let _homes = SandboxedHomes::new(root.path());
    for name in HOMES {
        fs::write(root.path().join(name.to_lowercase()), b"blocker").expect("file blocker");
    }
    let panels = TempDir::new().expect("panels");
    let mut app = app_in(panels.path());
    app.config.system.auto_save_setup = true;
    app.performance.record(
        "OpenHelp",
        std::time::Duration::from_millis(2),
        std::time::Duration::from_millis(1),
    );
    crate::recording::set_stopped_for_test("disk full");

    super::at_exit::end_session_after_wait(
        &app,
        Err(io::Error::other("lost")),
        Err(io::Error::other("cleanup")),
        true,
    )
    .expect("lost terminal suppresses error");
    assert!(super::at_exit::end_session(&app, Err(io::Error::other("boom")), Ok(())).is_err());
}

#[test]
fn exit_preserves_an_invalid_settings_file_when_auto_save_is_enabled() {
    let root = TempDir::new().expect("root");
    let _homes = SandboxedHomes::new(root.path());
    let settings = crate::config::Settings::path().expect("settings path");
    fs::create_dir_all(settings.parent().expect("settings directory")).expect("directory");
    fs::write(&settings, "[broken yaml").expect("settings");
    let mut app = app_in(root.path());
    app.config.system.auto_save_setup = true;
    super::at_exit::persist_at_exit(&app);
    assert_eq!(
        fs::read_to_string(settings).expect("settings"),
        "[broken yaml"
    );
}
