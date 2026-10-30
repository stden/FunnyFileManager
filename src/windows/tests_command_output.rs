//! Purpose: Tests for how a command's exit status and output streams become
//! a report or an error.

use super::report::classify_command_output;

// ── classify_command_output mutation killers ──────────────────

#[test]
fn test_classify_success_returns_stdout() {
    // Kill: replace success branch with Err
    let result = classify_command_output(true, "hello", "");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_classify_success_ignores_stderr() {
    // Kill: swapping stdout/stderr in success branch
    let result = classify_command_output(true, "good", "noise");
    assert_eq!(result.unwrap(), "good");
}

#[test]
fn test_classify_failure_with_stderr_uses_stderr_message() {
    // Kill: replace `!stderr.is_empty()` with `stderr.is_empty()`
    let result = classify_command_output(false, "", "bad thing happened");
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "bad thing happened");
}

#[test]
fn test_classify_failure_empty_stderr_uses_generic_message() {
    // Kill: replace empty-stderr branch with Ok or wrong message
    let result = classify_command_output(false, "", "");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("failure status"),
        "expected generic message, got: {err}"
    );
}

#[test]
fn test_classify_failure_stderr_not_generic() {
    // Kill: always using generic message regardless of stderr
    let result = classify_command_output(false, "out", "specific error");
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "specific error");
    assert!(
        !err.to_string().contains("failure status"),
        "should use stderr, not generic message"
    );
}

#[test]
fn test_classify_failure_empty_stderr_not_stderr_content() {
    // Kill: using stderr when it's empty instead of generic message
    let result = classify_command_output(false, "out", "");
    let err = result.unwrap_err();
    assert_ne!(
        err.to_string().len(),
        0,
        "error message should not be empty"
    );
    assert!(err.to_string().contains("failure status"));
}
