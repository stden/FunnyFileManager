//! Purpose: Preserve command configuration and surface failed private launches.

use super::*;
use std::ffi::{OsStr, OsString};

#[test]
fn private_command_preserves_arguments_cwd_and_environment_changes() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut command = Command::new("fixture-program");
    command
        .args(["argument with spaces", "кириллица"])
        .current_dir(directory.path())
        .env("FFM_PRIVATE_ASSIGNED", "fixture value")
        .env_remove("FFM_PRIVATE_REMOVED")
        .env("TERM", "inherited-terminal");
    let configured = builder(&command);
    assert_eq!(
        configured.get_argv(),
        &["fixture-program", "argument with spaces", "кириллица"]
    );
    assert_eq!(
        configured.get_cwd().map(OsString::as_os_str),
        Some(directory.path().as_os_str())
    );
    assert_eq!(
        configured.get_env("FFM_PRIVATE_ASSIGNED"),
        Some(OsStr::new("fixture value"))
    );
    assert_eq!(configured.get_env("FFM_PRIVATE_REMOVED"), None);
    assert_eq!(
        configured.get_env("TERM"),
        Some(OsStr::new("xterm-256color"))
    );
}

#[test]
fn unavailable_program_records_failed_state_and_can_be_reset() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let command = Command::new(directory.path().join("nonexistent-private-program"));
    let Err(error) = run(&host, &command, None, Descendants::Owned) else {
        panic!("nonexistent private program unexpectedly started");
    };
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error
        .to_string()
        .starts_with("Cannot create private terminal: "));
    assert_eq!(host.snapshot().state, ExternalState::Failed);
    host.reset().unwrap();
    assert_eq!(host.snapshot().state, ExternalState::Idle);
}

#[test]
fn a_file_cannot_be_used_as_a_private_launch_directory() {
    let root = tempfile::TempDir::new().unwrap();
    let file = root.path().join("file");
    std::fs::write(&file, b"keep").unwrap();
    let host = HeadlessHost::new();
    let mut command = Command::new("program-that-must-not-start");
    command.current_dir(&file);
    let error = run(&host, &command, None, Descendants::Owned).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
    assert_eq!(host.snapshot().state, ExternalState::Failed);
    assert_eq!(std::fs::read(file).unwrap(), b"keep");
}
