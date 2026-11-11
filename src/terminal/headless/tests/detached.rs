//! Purpose: Detached launchers still require a session and remain bounded before successful exit.

use super::*;
use std::time::Duration;

#[test]
fn detached_commands_require_an_installed_private_terminal() {
    let command = Command::new("ffm-nonexistent-fixture");
    assert_eq!(
        run_detached(&command, Duration::from_secs(1))
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotConnected
    );
}

#[test]
fn successful_detached_launcher_keeps_its_output_and_exit_status() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut command = fixture("echo detached-fixture");
    command.current_dir(directory.path());
    let outcome = run_detached(&command, Duration::from_secs(20)).unwrap();
    assert_eq!(outcome.code, 0);
    assert!(String::from_utf8_lossy(&outcome.output).contains("detached-fixture"));
    assert_eq!(host.snapshot().state, ExternalState::Exited { code: 0 });
}

#[test]
fn unfinished_detached_launcher_is_stopped_at_its_deadline() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    #[cfg(windows)]
    let mut command = fixture("Start-Sleep -Seconds 30");
    #[cfg(not(windows))]
    let mut command = fixture("sleep 30");
    command.current_dir(directory.path());
    assert_eq!(
        run_detached(&command, Duration::from_millis(20))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(host.snapshot().state, ExternalState::Failed);
}

fn fixture(script: &str) -> Command {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.arg("-c");
        command
    };
    command.arg(script);
    command
}
