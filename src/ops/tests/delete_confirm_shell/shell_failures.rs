//! Purpose: Shell commands that cannot start, or end with nothing on stdout.

use super::*;

// -----------------------------------------------------------------------
// Shell command execution error (nonexistent binary)
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_nonexistent_binary() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    // Point to nonexistent dir so the shell command's current_dir fails
    app.left_panel.path = PathBuf::from("/nonexistent_dir_99999");

    let result = execute_command(&mut app, "echo hello");
    assert!(result.is_err());
    Ok(())
}

// -----------------------------------------------------------------------
// Shell command via execute_command: stderr, exit code, error
// -----------------------------------------------------------------------

#[test]
fn test_shell_command_stderr_only_output() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that writes to stderr only
    execute_command(&mut app, "echo error_msg >&2")?;
    assert!(
        app.status_text().contains("Err:") && app.status_text().contains("error_msg"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_shell_command_no_output_success() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command with no output
    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().starts_with("OK:"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_shell_command_no_output_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // Command that fails silently
    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().starts_with("Exit"),
        "status: {}",
        app.status_text()
    );
    Ok(())
}
