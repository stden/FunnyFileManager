//! Purpose: The cd and bookmark commands.

use super::*;

#[test]
fn test_command_line_cd_absolute() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target = temp.path().to_path_buf();

    let mut app = App::new()?;

    app.set_overlay(Overlay::CommandLine(
        format!("cd {}", target.display()).into(),
    ));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_command_line_cd_relative() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("child");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    app.set_overlay(Overlay::CommandLine("cd child".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened"));

    Ok(())
}

#[test]
fn test_command_line_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Set bookmark 3
    app.set_overlay(Overlay::CommandLine("bookmark set 3".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Bookmark 3 set"));

    // Go to bookmark 3
    app.set_overlay(Overlay::CommandLine("bookmark go 3".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Opened bookmark 3"));

    Ok(())
}

#[test]
fn test_command_line_bookmark_errors() -> io::Result<()> {
    let mut app = App::new()?;

    // Missing subcommand
    app.set_overlay(Overlay::CommandLine("bookmark".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("bookmark"));

    // Missing slot
    app.set_overlay(Overlay::CommandLine("bookmark set".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("slot"));

    // Invalid slot
    app.set_overlay(Overlay::CommandLine("bookmark go 99".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("slot"));

    // Go to empty bookmark
    app.set_overlay(Overlay::CommandLine("bookmark go 0".into()));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(app.status_text().contains("Error") || app.status_text().contains("empty"));

    Ok(())
}
