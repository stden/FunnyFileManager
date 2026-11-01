//! Purpose: Headless commands preserve shell syntax and cancellation semantics.

use super::*;
use crate::terminal::headless::{ExternalState, HeadlessHost};

#[test]
fn commands_preserve_quoted_arguments_and_capture_both_streams() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    #[cfg(windows)]
    let command = "echo \"two words\" & echo error-output 1>&2 & exit /b 7";
    #[cfg(not(windows))]
    let command = "printf '%s\\n' \"two words\"; printf 'error-output\\n' >&2; exit 7";
    let output = run(command, directory.path(), Some(Duration::from_secs(15))).unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("two words"));
    assert!(text.contains("error-output"));
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(host.snapshot().state, ExternalState::Exited { code: 7 });
}

#[test]
fn command_timeout_ends_without_reading_the_host_terminal() {
    let directory = tempfile::TempDir::new().unwrap();
    let host = HeadlessHost::new();
    let _scope = host.enter();
    #[cfg(windows)]
    let command = "ping -n 60 127.0.0.1 >nul";
    #[cfg(not(windows))]
    let command = "sleep 60";
    let error = run(command, directory.path(), Some(Duration::from_millis(50))).unwrap_err();
    assert!(matches!(error, ShellError::TimedOut(_)));
    assert_eq!(host.snapshot().state, ExternalState::Failed);
}
