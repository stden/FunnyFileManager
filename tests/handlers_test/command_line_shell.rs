//! Purpose: Shell commands from the command line: output, silence and failure.

use super::*;

#[test]
fn test_command_line_shell_stdout() -> io::Result<()> {
    // Kill: stdout branch — last line of stdout shown as status
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine("echo hello".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().contains("hello"),
        "status: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
fn test_command_line_shell_silent_success() -> io::Result<()> {
    // Kill: silent-success branch — "OK: <cmd>" shown when no stdout/stderr, exit 0
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    // "rem" is a CMD remark (Windows) / comment (Unix sh) — exits 0, no output,
    // and is NOT intercepted by any internal command handler.
    #[cfg(windows)]
    let cmd = "rem silence";
    #[cfg(not(windows))]
    let cmd = "true";

    app.set_overlay(Overlay::CommandLine(cmd.into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().starts_with("OK:"),
        "expected 'OK:' prefix, got: {}",
        app.status_text()
    );

    Ok(())
}

#[test]
#[cfg(not(windows))]
fn test_command_line_shell_exit_failure() -> io::Result<()> {
    // Kill: exit-failure branch — "Exit <code>" shown when non-zero exit, no output
    // Unix-only: `false` exits 1 with no stdout/stderr and is not an internal command.
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

    app.set_overlay(Overlay::CommandLine("false".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        app.status_text().starts_with("Exit"),
        "expected 'Exit' prefix, got: {}",
        app.status_text()
    );

    Ok(())
}
