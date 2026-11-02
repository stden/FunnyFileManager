//! Purpose: Drive entries, and switching and swapping the app's panels.

use super::*;

// ---------------------------------------------------------------------------
// DriveEntry::display
// ---------------------------------------------------------------------------

#[test]
fn test_drive_entry_display_local() {
    let drive = DriveEntry::Local(PathBuf::from("/"));
    assert_eq!(drive.display(), "/");

    let drive2 = DriveEntry::Local(PathBuf::from("/home/user"));
    assert_eq!(drive2.display(), "/home/user");
}

#[test]
fn test_drive_entry_display_s3() {
    let drive = DriveEntry::S3Profile("production".to_owned());
    assert_eq!(drive.display(), "S3: production");

    let drive2 = DriveEntry::S3Profile("default".to_owned());
    assert_eq!(drive2.display(), "S3: default");
}

// ---------------------------------------------------------------------------
// App methods
// ---------------------------------------------------------------------------

#[test]
fn test_app_switch_panel() {
    let mut app = App::new().unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn test_app_get_active_panel_left() {
    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Left;
    let path = app.get_active_panel().path.clone();
    assert_eq!(path, app.left_panel.path);
}

#[test]
fn test_app_get_active_panel_right() {
    let mut app = App::new().unwrap();
    app.active_panel = PanelSide::Right;
    let path = app.get_active_panel().path.clone();
    assert_eq!(path, app.right_panel.path);
}

#[test]
fn test_app_swap_panels_exchanges_paths() {
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(temp1.path().to_path_buf()).unwrap();
    app.right_panel = Panel::new(temp2.path().to_path_buf()).unwrap();

    let left_path = app.left_panel.path.clone();
    let right_path = app.right_panel.path.clone();

    app.swap_panels();

    assert_eq!(app.left_panel.path, right_path);
    assert_eq!(app.right_panel.path, left_path);
}
