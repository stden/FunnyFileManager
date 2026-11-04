//! Purpose: Unit tests for bounded fish completion spawning, exit status handling, and timeout kill.

use std::path::Path;
use tempfile::TempDir;

use super::{fish_path_complete, wait_for_fish, FISH_PROGRAM_FOR_TESTS};
use crate::complete::{paths::path_complete, PathSyntax};

struct FishStubGuard;

impl Drop for FishStubGuard {
    fn drop(&mut self) {
        FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);
    }
}

fn with_stub<T>(program: &Path, run: impl FnOnce() -> T) -> T {
    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(program.to_path_buf()));
    let _guard = FishStubGuard;
    run()
}

#[cfg(windows)]
fn write_stub(dir: &Path, name: &str, win_body: &str, _unix_body: &str) -> std::path::PathBuf {
    let path = dir.join(format!("{name}.cmd"));
    std::fs::write(&path, format!("@echo off\r\n{win_body}\r\n")).unwrap();
    path
}

#[cfg(not(windows))]
fn write_stub(dir: &Path, name: &str, _win_body: &str, unix_body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    crate::test_support::write_executable(&path, format!("#!/bin/sh\n{unix_body}\n").as_bytes())
        .unwrap();
    path
}

#[test]
fn fish_path_complete_parses_stub_candidates_and_falls_back_on_error() {
    let dir = TempDir::new().unwrap();
    let ok_stub = write_stub(
        dir.path(),
        "fish_ok",
        "echo alpha/\tdir\r\necho beta/\tdir\r\nexit /b 0",
        "printf 'alpha/\\tdir\\nbeta/\\tdir\\n'",
    );
    let got = with_stub(&ok_stub, || fish_path_complete("a", dir.path()));
    assert_eq!(got, Some(vec!["alpha/".to_owned(), "beta/".to_owned()]));
    let merged = with_stub(&ok_stub, || {
        path_complete(PathSyntax::HOST, "a", dir.path())
    });
    assert!(merged.contains(&"alpha/".to_owned()));
    assert!(merged.contains(&"beta/".to_owned()));

    let fail_stub = write_stub(dir.path(), "fish_fail", "exit /b 2", "exit 2");
    assert_eq!(
        with_stub(&fail_stub, || fish_path_complete("a", dir.path())),
        None
    );
}

#[test]
fn fish_path_complete_kills_hung_process_after_timeout() {
    let dir = TempDir::new().unwrap();
    let slow_stub = write_stub(
        dir.path(),
        "fish_slow",
        "ping.exe -n 5 127.0.0.1 >nul",
        "sleep 5",
    );
    assert_eq!(
        with_stub(&slow_stub, || fish_path_complete("a", dir.path())),
        None
    );
}

#[test]
fn wait_for_fish_logs_and_survives_unreadable_stdout_spool() {
    let dir = TempDir::new().unwrap();
    let ok_stub = write_stub(dir.path(), "fish_spool", "exit /b 0", "exit 0");
    let spool_path = dir.path().join("write_only_spool.txt");
    let write_only = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&spool_path)
        .unwrap();
    let mut child = std::process::Command::new(&ok_stub).spawn().unwrap();
    assert_eq!(wait_for_fish(&mut child, write_only), None);
}

struct PathAndMemoGuard(Option<std::ffi::OsString>);

impl Drop for PathAndMemoGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(prev) => std::env::set_var("PATH", prev),
            None => std::env::remove_var("PATH"),
        }
        super::reset_fish_unavailable();
    }
}

#[test]
fn fish_unavailable_memoizes_not_found_and_bypasses_memo_for_test_stubs() {
    let dir = TempDir::new().unwrap();
    assert_eq!(
        with_stub(&dir.path().join("missing_stub_bin"), || {
            fish_path_complete("a", dir.path())
        }),
        None
    );

    let _lock = crate::test_support::env_lock();
    let _restore = PathAndMemoGuard(std::env::var_os("PATH"));
    super::reset_fish_unavailable();
    std::env::set_var("PATH", dir.path());

    assert_eq!(fish_path_complete("a", dir.path()), None);
    assert!(super::FISH_UNAVAILABLE.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(fish_path_complete("a", dir.path()), None);
}
