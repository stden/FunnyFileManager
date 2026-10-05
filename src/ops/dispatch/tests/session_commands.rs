//! Purpose: help, quit and exit, invalid input, shell commands and `:mcp`.

use super::*;

// ─── help / quit ─────────────────────────────────────────────────────

#[test]
fn test_help_opens_overlay() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "help").expect("help");
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn test_quit_sets_flag() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "quit").expect("quit");
    assert!(!app.lifecycle.is_running());
}

#[test]
fn test_exit_sets_flag() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "exit").expect("exit");
    assert!(!app.lifecycle.is_running());
}

// ─── invalid ─────────────────────────────────────────────────────────

#[test]
fn test_invalid_command_errors() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    let result = execute_command(&mut app, "sort garbage");
    assert!(result.is_err());
}

// ─── shell ───────────────────────────────────────────────────────────

#[test]
fn test_shell_echo_stdout() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "echo world").expect("shell");
    assert!(app.status_text().contains("world"));
}

#[test]
fn test_shell_empty_errors() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    // Empty shell (just spaces) should fail
    let result = execute_command(&mut app, "   ");
    // The parser treats whitespace-only as Shell("") which is empty
    // Actually split_whitespace returns None for "   ", so it's Shell("")
    // Then the Shell arm checks is_empty and returns InvalidInput
    assert!(result.is_err());
}

#[test]
fn test_shell_stderr_shows_err_prefix() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    #[cfg(windows)]
    let cmd = "echo oops 1>&2";
    #[cfg(not(windows))]
    let cmd = "echo oops >&2";
    execute_command(&mut app, cmd).expect("shell");
    assert!(app.status_text().starts_with("Err:"));
    assert!(app.status_text().contains("oops"));
}

#[test]
fn test_shell_success_no_output_shows_ok() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    #[cfg(windows)]
    let cmd = "ver >nul 2>nul";
    #[cfg(not(windows))]
    let cmd = "true";
    execute_command(&mut app, cmd).expect("shell");
    assert!(
        app.status_text().starts_with("OK:"),
        "expected OK: prefix, got: {}",
        app.status_text()
    );
}

#[test]
fn test_shell_failure_shows_exit() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    #[cfg(windows)]
    let cmd = "cmd /c exit /b 1";
    #[cfg(not(windows))]
    let cmd = "false";
    execute_command(&mut app, cmd).expect("shell");
    assert!(
        app.status_text().starts_with("Exit"),
        "expected Exit prefix, got: {}",
        app.status_text()
    );
}

// ─── mcp ─────────────────────────────────────────────────────────────

#[test]
fn test_mcp_opens_viewer() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);
    execute_command(&mut app, "mcp").expect("mcp");
    assert!(app.get_viewer().is_some());
    assert!(app.status_text().contains("MCP"));
}
