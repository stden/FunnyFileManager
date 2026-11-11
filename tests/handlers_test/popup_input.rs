//! Purpose: Popup input: cd .., viewer scrolling and closing, and the help toggle.

use super::*;

#[test]
fn test_cd_dotdot() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(subdir)?;

    app.set_overlay(Overlay::CommandLine("cd ..".into()));
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        app.left_panel.path.canonicalize()?,
        temp_dir.path().canonicalize()?
    );

    Ok(())
}

#[test]
fn test_viewer_scroll_down_up() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec![
            "Line 1".to_owned(),
            "Line 2".to_owned(),
            "Line 3".to_owned(),
            "Line 4".to_owned(),
            "Line 5".to_owned(),
        ],
    )));
    app.set_viewport(funny_file_manager::models::Viewport::new(2));

    // Scroll down
    process_popup_input(&mut app, key_event(KeyCode::Down));
    assert_eq!(app.get_viewer().unwrap().scroll.lines(), 1);

    // Scroll up
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    // Scroll up at 0 stays at 0
    process_popup_input(&mut app, key_event(KeyCode::Up));
    assert_eq!(app.get_viewer().unwrap().scroll, ScrollTop::TOP);

    Ok(())
}

#[test]
fn test_viewer_close_with_f3() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "test.txt",
        vec!["Line 1".to_owned()],
    )));

    process_popup_input(&mut app, key_event(KeyCode::F(3)));
    assert!(app.get_viewer().is_none());

    Ok(())
}

#[test]
fn test_help_toggle_with_f1() -> io::Result<()> {
    let mut app = App::new()?;
    app.set_overlay(Overlay::Help);

    process_popup_input(&mut app, key_event(KeyCode::F(1)));
    assert!(!matches!(app.overlay, Overlay::Help));

    Ok(())
}
