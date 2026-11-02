//! Purpose: Unit tests for bounded fish completion spawning, exit status handling, and timeout kill.

use std::path::Path;
use tempfile::TempDir;

use super::{fish_path_complete, FISH_PROGRAM_FOR_TESTS};

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
    crate::test_support::write_executable(&path, &format!("#!/bin/sh\n{unix_body}\n")).unwrap();
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
