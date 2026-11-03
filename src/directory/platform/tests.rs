//! Unit tests: platform-specific entry classification helpers.

use super::{allocated_bytes, is_executable, is_hidden};
use tempfile::TempDir;

#[test]
fn allocated_bytes_is_positive_for_a_non_empty_file() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("data.txt");
    std::fs::write(&path, b"hello world").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(allocated_bytes(&meta).bytes() > 0);
}

#[test]
fn allocated_bytes_is_zero_or_positive_for_an_empty_file() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("empty.txt");
    std::fs::write(&path, b"").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    // An empty file may get 0 bytes allocated on some filesystems.
    let _ = allocated_bytes(&meta);
}

// ── executability ────────────────────────────────────────────────────────────

#[test]
fn is_executable_false_for_txt_file() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, b"").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(!is_executable("notes.txt", &meta));
}

#[test]
fn is_executable_false_for_a_directory() {
    let dir = TempDir::new().expect("temp");
    let meta = std::fs::metadata(dir.path()).expect("meta");
    // A directory is not executable in the panel's sense even if it has runnable
    // extension markers — is_executable is defined only for files.
    assert!(!is_executable("script.bat", &meta));
}

#[test]
fn is_executable_false_for_no_extension() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("makefile");
    std::fs::write(&path, b"").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(!is_executable("makefile", &meta));
}

// ── platform-specific: runnable extension (non-unix) / execute bit (unix) ───

#[cfg(not(unix))]
#[test]
fn is_executable_true_for_bat_on_windows() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("run.bat");
    std::fs::write(&path, b"@echo off").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(is_executable("run.bat", &meta));
}

#[cfg(not(unix))]
#[test]
fn is_executable_true_for_exe_on_windows() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("app.exe");
    std::fs::write(&path, b"MZ").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(is_executable("app.exe", &meta));
}

#[cfg(unix)]
#[test]
fn is_executable_true_for_sh_script_on_unix() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("build.sh");
    std::fs::write(&path, b"#!/bin/sh").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(is_executable("build.sh", &meta));
}

#[cfg(unix)]
#[test]
fn is_executable_true_when_execute_bit_is_set() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("binary");
    std::fs::write(&path, b"\x7fELF").expect("write");
    let mut perms = std::fs::metadata(&path).expect("meta").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod");
    let meta = std::fs::metadata(&path).expect("meta");
    assert!(is_executable("binary", &meta));
}

// ── hidden flag ──────────────────────────────────────────────────────────────

#[test]
fn is_hidden_false_for_regular_file() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("visible.txt");
    std::fs::write(&path, b"").expect("write");
    let meta = std::fs::metadata(&path).expect("meta");
    // On Unix this is always false; on Windows a plain new file is not hidden.
    assert!(!is_hidden(&meta));
}
