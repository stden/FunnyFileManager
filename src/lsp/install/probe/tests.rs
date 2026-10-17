//! Purpose: Pin that a probe sees only the machine it was given: its own
//! `PATH` and cache, never the process's.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::lsp::install::launch;

fn path_of(dirs: &[&Path]) -> Option<OsString> {
    execs::join_paths(dirs.iter().copied())
}

fn program(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(launch::executable(name));
    crate::test_support::write_executable(&path, b"program").expect("program");
    path
}

#[test]
fn a_probe_finds_a_program_only_on_its_own_path() {
    let bin = TempDir::new().expect("bin");
    let other = TempDir::new().expect("other");
    let tool = program(bin.path(), "tool");
    let probe = Probe::new(None, path_of(&[bin.path()]));
    assert_eq!(probe.program(&["tool"], "missing"), Ok(tool));
    let elsewhere = Probe::new(None, path_of(&[other.path()]));
    assert_eq!(
        elsewhere.program(&["tool"], "missing"),
        Err(InstallError::Missing("missing"))
    );
}

#[test]
fn the_default_probe_knows_nothing() {
    let probe = Probe::default();
    assert!(probe.programs().is_empty());
    assert_eq!(probe.cache(), None);
    assert_eq!(
        probe.program(&["sh"], "missing"),
        Err(InstallError::Missing("missing"))
    );
    probe.warm();
    assert!(
        probe.programs().is_empty(),
        "warming a listed probe scans nothing"
    );
}

/// A `PATH` entry that is missing, a file, or a directory that cannot be
/// listed contributes nothing; the entries after it still count.
#[test]
fn a_listed_probe_offers_its_scan_and_skips_path_entries_it_cannot_read() {
    let home = TempDir::new().expect("home");
    let bin = home.path().join("bin");
    fs::create_dir(&bin).expect("bin");
    program(&bin, "zeta");
    program(&bin, "alpha");
    let file = home.path().join("a-file");
    fs::write(&file, "not a directory").expect("file");
    let missing = home.path().join("missing");
    let unreadable = unreadable_dir(home.path());
    let probe = Probe::new(
        Some(home.path().join("cache")),
        path_of(&[&missing, &file, unreadable.as_ref(), &bin]),
    );
    assert_eq!(
        probe.programs(),
        [launch::executable("alpha"), launch::executable("zeta")]
    );
    assert_eq!(probe.cache(), Some(home.path().join("cache").as_path()));
    assert!(probe.program(&["alpha"], "missing").is_ok());
}

/// A directory this user may not list while the guard lives; dropping it
/// gives the mode back, so `home` can be removed. Root lists anything, so
/// there it is merely an empty directory, which the scan skips the same way.
#[cfg(unix)]
fn unreadable_dir(home: &Path) -> crate::test_support::ModeGuard {
    let dir = home.join("locked");
    fs::create_dir(&dir).expect("locked");
    crate::test_support::ModeGuard::set(&dir, 0o000).expect("chmod")
}

#[cfg(not(unix))]
fn unreadable_dir(home: &Path) -> PathBuf {
    home.join("never-created")
}

/// Availability only inspects: a probe refuses to run anything.
#[test]
fn a_probe_never_runs_a_program() {
    let bin = TempDir::new().expect("bin");
    let tool = program(bin.path(), "tool");
    let probe = Probe::new(None, path_of(&[bin.path()]));
    assert_eq!(
        probe.run(&mut Command::new(tool), &AtomicBool::new(false)),
        Err(InstallError::Command)
    );
}

/// The production probe reads the process's cache location and lists `PATH`
/// programs without ever blocking on the scan.
#[test]
fn the_process_probe_uses_the_config_cache_and_never_blocks() {
    let probe = Probe::from_process();
    assert_eq!(probe.cache(), super::super::cache_dir().as_deref());
    let _listed_now = probe.programs();
    probe.warm();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while execs::ready().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "the PATH scan finishes"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(probe.programs(), execs::executables());
}

#[test]
fn test_support_hands_an_app_a_fake_machine() {
    let home = TempDir::new().expect("home");
    program(home.path(), "pylsp");
    let panel = crate::models::Panel::from_entries(home.path().to_owned(), Vec::new());
    let mut app = crate::models::App::with_panels(panel.clone(), panel);
    crate::test_support::set_lsp_probe(&mut app, &home.path().join("cache"), &[home.path()]);
    assert_eq!(
        app.live.lsp_probe.cache(),
        Some(home.path().join("cache").as_path())
    );
    assert_eq!(app.live.lsp_probe.programs(), [launch::executable("pylsp")]);
}

#[cfg(unix)]
#[test]
fn every_name_is_launchable_on_unix() {
    assert!(launch::launchable_name("pylsp"));
    assert!(launch::launchable_name("server.sh"));
}

/// Windows lookup appends `.exe`, so only an `.exe` on `PATH` is offered.
#[cfg(windows)]
#[test]
fn only_an_exe_is_launchable_on_windows() {
    assert!(launch::launchable_name("pylsp.exe"));
    assert!(launch::launchable_name("PYLSP.EXE"));
    assert!(!launch::launchable_name("pylsp"));
    assert!(!launch::launchable_name("pylsp.cmd"));
    assert!(!launch::launchable_name("user32.dll"));
}
