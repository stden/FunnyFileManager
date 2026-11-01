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
