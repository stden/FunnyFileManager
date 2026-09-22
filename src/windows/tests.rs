use super::*;
use crate::scroll::ScrollTop;

// These tests use `.expect(...)` rather than `?`: the runners below cannot
// fail on a non-Windows host, so a `?` would leave an error branch that no
// test can ever reach.

#[test]
fn test_open_report_with_populates_viewer() {
    let mut app = App::new().expect("test setup");

    open_report_with(&mut app, SYSTEM_MESSAGES_TITLE, || {
        Ok("first line\nsecond line".to_string())
    })
    .expect("the report runner cannot fail");

    let viewer = app.get_viewer().expect("viewer should open");
    assert_eq!(viewer.title(), SYSTEM_MESSAGES_TITLE);
    assert_eq!(viewer.lines, vec!["first line", "second line"]);
    assert_eq!(app.status_text(), format!("Opened {SYSTEM_MESSAGES_TITLE}"));
}

#[test]
fn test_open_report_with_uses_placeholder_for_empty_output() {
    let mut app = App::new().expect("test setup");

    open_report_with(&mut app, PERFORMANCE_TITLE, || Ok(" \n\n".to_string()))
        .expect("the report runner cannot fail");

    let viewer = app.get_viewer().expect("viewer should open");
    assert_eq!(viewer.title(), PERFORMANCE_TITLE);
    assert_eq!(viewer.lines, vec!["<no data>"]);
}

#[test]
fn test_open_windows_system_messages_on_non_windows() {
    let mut app = App::new().expect("test setup");
    open_windows_system_messages(&mut app).expect("system messages report");

    #[cfg(not(windows))]
    assert!(app.status_text().contains("only available on Windows"));

    #[cfg(windows)]
    assert!(app.is_overlay_active() || app.status_text().contains("error"));
}

#[test]
fn test_open_windows_performance_checker_on_non_windows() {
    let mut app = App::new().expect("test setup");
    open_windows_performance_checker(&mut app).expect("performance report");

    #[cfg(not(windows))]
    assert!(app.status_text().contains("only available on Windows"));

    #[cfg(windows)]
    assert!(app.is_overlay_active() || app.status_text().contains("error"));
}

#[test]
fn test_open_report_with_error_propagates() {
    let mut app = App::new().expect("test setup");
    let result = open_report_with(&mut app, "Test", || Err(io::Error::other("test error")));
    assert!(result.is_err());
    assert!(!app.is_overlay_active());
}

#[test]
fn test_powershell_executable_is_explicit() {
    assert_eq!(powershell_executable(), "powershell.exe");
}

#[test]
fn test_powershell_report_args_include_expected_flags() {
    assert_eq!(
        powershell_report_args("Get-Date"),
        [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Get-Date"
        ]
    );
}

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
    assert!(
        !err.to_string().is_empty(),
        "error message should not be empty"
    );
    assert!(err.to_string().contains("failure status"));
}

// ── report_lines mutation killers ────────────────────────────

#[test]
fn test_report_lines_filters_blank_lines() {
    let lines = report_lines("first\n\n  \nsecond\n");
    assert_eq!(lines, vec!["first", "second"]);
}

#[test]
fn test_report_lines_trims_whitespace() {
    let lines = report_lines("  padded  ");
    assert_eq!(lines, vec!["padded"]);
}

#[test]
fn test_report_lines_empty_input_returns_placeholder() {
    // Kill: removing the is_empty check / placeholder push
    let lines = report_lines("");
    assert_eq!(lines, vec!["<no data>"]);
}

#[test]
fn test_report_lines_whitespace_only_returns_placeholder() {
    let lines = report_lines("   \n  \n  ");
    assert_eq!(lines, vec!["<no data>"]);
}

// ── open_report_viewer mutation killers ───────────────────────

#[test]
fn test_open_report_viewer_sets_scroll_to_zero() {
    let mut app = App::new().expect("test setup");
    open_report_viewer(&mut app, "Test", "line1\nline2");
    let viewer = app.get_viewer().expect("viewer");
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_open_report_viewer_sets_title() {
    let mut app = App::new().expect("test setup");
    open_report_viewer(&mut app, "My Report", "data");
    let viewer = app.get_viewer().expect("viewer");
    assert_eq!(viewer.title(), "My Report");
}

#[test]
fn test_open_report_viewer_sets_status_message() {
    let mut app = App::new().expect("test setup");
    open_report_viewer(&mut app, "Diag", "data");
    assert_eq!(app.status_text(), "Opened Diag");
}

#[cfg(windows)]
#[test]
fn test_run_powershell_report_failure_with_stderr() {
    let result = run_powershell_report("Write-Error 'something failed'");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failed") || err.to_string().contains("Error"));
}

#[cfg(windows)]
#[test]
fn test_run_powershell_report_failure_status_only() {
    let result = run_powershell_report("[System.Environment]::Exit(42)");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("PowerShell exited with status") || err.to_string().contains("42")
    );
}
