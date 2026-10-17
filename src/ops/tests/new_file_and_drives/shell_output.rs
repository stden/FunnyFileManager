//! Purpose: What a shell command prints, and an empty bookmark, on the status bar.

use super::*;

#[test]
fn test_execute_command_bookmark_go_empty_slot_returns_error() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    // Slot 5 is empty by default.
    assert!(app
        .bookmarks
        .get(BookmarkSlot::from_usize(5).expect("slot"))
        .is_none());

    let result = execute_command(&mut app, "bookmark go 5");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("empty") || msg.contains("bookmark"),
        "error message should reference bookmark"
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Shell command execution -- stdout / stderr / success / exit code branches
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_stdout_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "echo hello" produces stdout
    execute_command(&mut app, "echo hello")?;
    assert_eq!(app.status_text(), "hello");

    Ok(())
}

#[test]
fn test_shell_command_stdout_multiline_uses_last() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // printf produces multi-line stdout; status should be the last line
    execute_command(&mut app, "printf 'line1\\nline2\\nline3'")?;
    assert_eq!(app.status_text(), "line3");

    Ok(())
}

#[test]
fn test_shell_command_stderr_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that writes to stderr but no stdout
    execute_command(&mut app, "echo errmsg >&2 && true")?;
    assert!(
        app.status_text().contains("Err:"),
        "stderr output should be prefixed with 'Err:': {}",
        app.status_text()
    );
    assert!(
        app.status_text().contains("errmsg"),
        "stderr content should appear in status: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_shell_command_success_no_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "true" produces no output and exits 0
    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().starts_with("OK:"),
        "success with no output should show 'OK: ...': {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_shell_command_exit_code_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "false" produces no output and exits non-zero
    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().starts_with("Exit"),
        "failure with no output should show 'Exit ...': {}",
        app.status_text()
    );

    Ok(())
}
