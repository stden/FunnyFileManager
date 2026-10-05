//! Purpose: Tests for the messages a failed or completed command leaves.

use super::*;

#[test]
fn package_manager_failure_is_an_error_not_a_success_message() {
    let result = execute_with(
        &WslOperation::UpdatePackages(distribution("Custom")),
        |_, _| {
            let mut captured = success("");
            captured.exit = Exit::Failed(Some(127));
            captured.stderr.bytes = b"No supported package manager found.\n".to_vec();
            Ok(captured)
        },
    );
    assert_eq!(
        result.unwrap_err(),
        "Package update for Custom failed (exit code 127).\nNo supported package manager found."
    );
}

#[test]
fn command_failures_keep_stderr_and_exit_code_when_output_is_empty() {
    let result = execute_with(&WslOperation::Refresh, |_, _| {
        let mut captured = success("Details");
        captured.exit = Exit::Failed(Some(1));
        captured.stderr.bytes = utf16le("WSL is not installed.\r\n");
        Ok(captured)
    });
    assert_eq!(
        result.unwrap_err(),
        "WSL distribution refresh failed (exit code 1).\nWSL is not installed.\nDetails"
    );

    let result = execute_with(&WslOperation::Update, |_, _| {
        let mut captured = success("");
        captured.exit = Exit::Failed(Some(42));
        Ok(captured)
    });
    assert!(result.unwrap_err().contains("42"));
}

#[test]
fn runner_failure_is_propagated() {
    assert_eq!(
        execute_with(&WslOperation::Refresh, |_, _| Err(
            "Cannot start WSL".to_owned()
        ))
        .unwrap_err(),
        "Cannot start WSL"
    );
}

#[test]
fn completed_updates_report_empty_and_truncated_output() {
    assert_eq!(
        execute_with(&WslOperation::Update, |_, _| Ok(success(""))).unwrap(),
        WslOutput::Updated("WSL update completed.".to_owned())
    );
    let result = execute_with(
        &WslOperation::UpdatePackages(distribution("Ubuntu")),
        |_, _| {
            let mut captured = success("Installing packages");
            captured.stdout.truncated = true;
            Ok(captured)
        },
    )
    .unwrap();
    let WslOutput::PackagesUpdated(message) = result else {
        panic!("expected package update result");
    };
    assert_eq!(
        message.lines().next(),
        Some("Package update completed: Ubuntu.")
    );
    assert!(message.contains("Installing packages"));
    assert!(message.contains("output truncated; command continued to completion"));
}

#[test]
fn completed_updates_with_undecodable_output_are_errors_not_successes() {
    for operation in [
        WslOperation::Update,
        WslOperation::UpdatePackages(distribution("Ubuntu")),
    ] {
        let error = execute_with(&operation, |_, _| Ok(success(vec![0xff]))).unwrap_err();
        assert_eq!(error, "WSL returned invalid text encoding");
    }
}

#[test]
fn a_failure_without_an_exit_code_says_the_status_is_unknown() {
    let result = execute_with(&WslOperation::Update, |_, _| {
        let mut captured = success("");
        captured.exit = Exit::Failed(None);
        Ok(captured)
    });
    assert_eq!(
        result.unwrap_err(),
        "WSL update failed (unknown exit status)."
    );
}
