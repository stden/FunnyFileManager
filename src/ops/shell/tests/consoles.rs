//! Purpose: Console requests use private headless I/O or report unsupported hosts.

use super::*;
use crate::terminal::headless::HeadlessHost;

#[cfg(not(windows))]
#[test]
fn a_separate_interactive_console_is_refused_without_starting_the_command() {
    let root = TempDir::new().unwrap();
    let command = ShellCommand::new("echo unexpected > marker").unwrap();
    let error = spawn_console(&command, root.path()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(!root.path().join("marker").exists());
}

#[test]
fn a_headless_console_executes_in_the_requested_directory() {
    let root = TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let command = ShellCommand::new("echo private-console > marker").unwrap();
    spawn_console(&command, root.path()).unwrap();
    assert!(std::fs::read_to_string(root.path().join("marker"))
        .unwrap()
        .contains("private-console"));
    let output =
        run_shell_command_with_timeout("echo private-shell", root.path(), Duration::from_secs(5))
            .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("private-shell"));
}

#[test]
fn a_failed_headless_console_names_its_launch_directory() {
    let root = TempDir::new().unwrap();
    let missing = root.path().join("removed");
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let command = ShellCommand::new("echo should-not-run").unwrap();
    let error = spawn_console(&command, &missing).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error.to_string().contains("removed"));
    assert!(!missing.exists());
}
