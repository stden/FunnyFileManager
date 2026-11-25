//! Purpose: What a `:` shell command's stdout, stderr and exit status put on the status bar.

use super::*;

#[test]
fn test_execute_command_shell_stdout() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "echo hello")?;
    assert!(
        app.status_text().contains("hello"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_stderr_only() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "echo error >&2")?;
    assert!(
        app.status_text().contains("Err"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_exit_nonzero() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "false")?;
    assert!(
        app.status_text().contains("Exit"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}

#[test]
fn test_execute_command_shell_ok_no_output() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;

    execute_command(&mut app, "true")?;
    assert!(
        app.status_text().contains("OK"),
        "got: {}",
        app.status_text()
    );
    Ok(())
}
