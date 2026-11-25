//! Purpose: Viewer scrolling, command-line editing and panel switching through popup input.

use super::*;

#[test]
fn test_viewer_scroll_limits() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(funny_file_manager::models::Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned(), "Line 2".to_owned()],
    )));

    // Try to scroll down beyond limit
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));
    process_popup_input(&mut app, key_event(KeyCode::Down, KeyModifiers::empty()));

    // Should stop at max scroll
    assert!(app.get_viewer().unwrap().scroll.lines() <= 1);

    Ok(())
}

#[test]
fn test_command_line_cd_subdir() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("cd subdir".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(app.left_panel.path.file_name().unwrap(), "subdir");

    Ok(())
}

#[test]
fn test_command_line_empty_enter() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.set_overlay(Overlay::CommandLine("".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter, KeyModifiers::empty()));

    assert!(app.get_command_line().is_none());

    Ok(())
}

#[test]
fn test_command_line_backspace() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::CommandLine("hello".into()));

    process_popup_input(
        &mut app,
        key_event(KeyCode::Backspace, KeyModifiers::empty()),
    );
    assert_eq!(app.get_command_line(), Some("hell"));

    Ok(())
}

#[test]
fn test_app_switch_panel() -> io::Result<()> {
    let mut app = App::new()?;

    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Left
    ));
    app.switch_panel();
    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Right
    ));
    app.switch_panel();
    assert!(matches!(
        app.active_panel,
        funny_file_manager::models::PanelSide::Left
    ));

    Ok(())
}
