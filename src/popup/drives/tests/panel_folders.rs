//! Purpose: which folder a chosen drive opens at — the folder a panel already has there, from either panel.

use super::*;

/// Far keeps a current directory per drive, so choosing the drive the active
/// panel is already on brings the other panel to that same folder, not to the
/// drive's root — the left panel deep in a project, Alt+F2, the same drive,
/// and the right panel lands beside it.
#[test]
fn selecting_the_active_panels_drive_opens_its_folder_in_the_other_panel() {
    let temp = TempDir::new().expect("temp dir");
    let project = temp.path().join("project");
    let elsewhere = temp.path().join("elsewhere");
    std::fs::create_dir(&project).expect("mkdir");
    std::fs::create_dir(&elsewhere).expect("mkdir");

    let mut app = App::new().expect("test app");
    app.active_panel = PanelSide::Left;
    app.left_panel = Panel::new(project.clone()).expect("panel");
    app.right_panel = Panel::new(elsewhere).expect("panel");

    let status = select_drive(
        &mut app,
        PanelSide::Right,
        DriveEntry::Local(temp.path().to_path_buf()),
    );

    assert_eq!(status, Status::ChangedTo(project.clone()));
    assert_eq!(app.right_panel.path, project);
}

/// The panel the menu was opened for is itself on that drive and is the active
/// one: it stays where it is instead of being thrown back to the root.
#[test]
fn selecting_the_drive_a_panel_is_already_on_keeps_its_folder() {
    let temp = TempDir::new().expect("temp dir");
    let project = temp.path().join("project");
    std::fs::create_dir(&project).expect("mkdir");

    let mut app = App::new().expect("test app");
    app.active_panel = PanelSide::Left;
    app.left_panel = Panel::new(project.clone()).expect("panel");

    let status = select_drive(
        &mut app,
        PanelSide::Left,
        DriveEntry::Local(temp.path().to_path_buf()),
    );

    assert_eq!(status, Status::ChangedTo(project.clone()));
    assert_eq!(app.left_panel.path, project);
}

/// When the active panel is elsewhere (another drive, an S3 profile) the other
/// panel's folder on that drive is the one remembered.
#[test]
fn selecting_a_drive_only_the_inactive_panel_is_on_opens_that_panels_folder() {
    let temp = TempDir::new().expect("temp dir");
    let project = temp.path().join("project");
    std::fs::create_dir(&project).expect("mkdir");

    let mut app = App::new().expect("test app");
    app.active_panel = PanelSide::Right;
    app.left_panel = Panel::new(project.clone()).expect("panel");

    let status = select_drive(
        &mut app,
        PanelSide::Right,
        DriveEntry::Local(temp.path().to_path_buf()),
    );

    assert_eq!(status, Status::ChangedTo(project.clone()));
    assert_eq!(app.right_panel.path, project);
}
