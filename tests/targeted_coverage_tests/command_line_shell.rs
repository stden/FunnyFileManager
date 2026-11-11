//! Purpose: Shell commands run from the command line: output, stderr and exit codes.

use super::*;

// ──────────────────────────────────────────────
// Shell command execution via command line
// ──────────────────────────────────────────────

#[test]
fn test_command_line_execute_shell_command() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Type a shell command: "echo hello"
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    for ch in "echo hello".chars() {
        process_popup_input(&mut app, key(KeyCode::Char(ch)));
    }
    assert_eq!(app.get_command_line(), Some("echo hello"));

    // Execute with Enter
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.get_command_line().is_none());
    // Shell command stdout should be in status
    assert!(
        app.status_text().contains("hello"),
        "Expected 'hello' in status, got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_stderr() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that produces stderr only
    app.set_overlay(Overlay::CommandLine(shell_stderr_command().into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Err") || app.status_text().contains("error"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_exit_code() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that exits with non-zero, no output
    app.set_overlay(Overlay::CommandLine(shell_failure_command().into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("Exit"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_ok_no_output() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Command that succeeds with no output
    app.set_overlay(Overlay::CommandLine(
        shell_success_no_output_command().into(),
    ));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(
        app.status_text().contains("OK"),
        "Got: {}",
        app.status_text()
    );

    Ok(())
}
