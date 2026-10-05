//! Purpose: unit tests for `PanelSide` and the app's active and inactive
//! panels.

use super::*;

#[test]
fn test_panel_side_equality() {
    assert_eq!(PanelSide::Left, PanelSide::Left);
    assert_eq!(PanelSide::Right, PanelSide::Right);
    assert_ne!(PanelSide::Left, PanelSide::Right);
    assert_ne!(PanelSide::Right, PanelSide::Left);
}

#[test]
fn test_panel_side_copy() {
    let side = PanelSide::Left;
    let copied = side;
    assert_eq!(side, copied);
}

#[test]
fn test_app_new() -> io::Result<()> {
    let app = App::new().expect("test setup");

    assert_eq!(app.active_panel, PanelSide::Left);
    assert!(!app.is_overlay_active());
    assert_eq!(app.bookmarks.slots().len(), 10);
    assert!(app.lifecycle.is_running());
    assert_eq!(
        app.status_text(),
        "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete"
    );

    // Both panels should be in the current directory
    assert_eq!(app.left_panel.path, app.right_panel.path);

    Ok(())
}

#[test]
fn test_app_get_active_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Initially active panel is left
    let left_path = app.left_panel.path.clone();
    let active_path = app.get_active_panel().path.clone();
    assert_eq!(active_path, left_path);

    // Switch to right panel
    app.active_panel = PanelSide::Right;
    let right_path = app.right_panel.path.clone();
    let active_path = app.get_active_panel().path.clone();
    assert_eq!(active_path, right_path);

    Ok(())
}

#[test]
fn test_app_panel_ref_getters() -> io::Result<()> {
    let temp_dir1 = TempDir::new().expect("test setup");
    let temp_dir2 = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir1.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir2.path().to_path_buf()).expect("test setup");

    app.active_panel = PanelSide::Left;
    assert_eq!(app.get_active_panel_ref().path, temp_dir1.path());
    assert_eq!(app.get_inactive_panel_ref().path, temp_dir2.path());

    app.active_panel = PanelSide::Right;
    assert_eq!(app.get_active_panel_ref().path, temp_dir2.path());
    assert_eq!(app.get_inactive_panel_ref().path, temp_dir1.path());

    Ok(())
}

#[test]
fn test_app_get_inactive_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Initially active panel is left, so inactive is right
    let right_path = app.right_panel.path.clone();
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, right_path);

    // Switch to right panel
    app.active_panel = PanelSide::Right;
    let left_path = app.left_panel.path.clone();
    let inactive_path = app.get_inactive_panel().path.clone();
    assert_eq!(inactive_path, left_path);

    Ok(())
}

#[test]
fn test_app_switch_panel() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    assert_eq!(app.active_panel, PanelSide::Left);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);

    Ok(())
}

#[test]
fn test_app_switch_panel_multiple_times() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    // Switch multiple times and ensure it alternates correctly
    for _ in 0..10 {
        let before = app.active_panel;
        app.switch_panel();
        let after = app.active_panel;

        match before {
            PanelSide::Left => assert_eq!(after, PanelSide::Right),
            PanelSide::Right => assert_eq!(after, PanelSide::Left),
        }
    }

    Ok(())
}
