//! Purpose: Drive entries, the drive list and the app's panel accessors.

use super::*;

#[test]
fn test_drive_entry_display_variants() {
    let local = DriveEntry::Local(PathBuf::from("/tmp"));
    let s3 = DriveEntry::S3Profile("default".to_owned());

    assert_eq!(local.display(), PathBuf::from("/tmp").display().to_string());
    assert_eq!(s3.display(), "S3: default");
}

#[test]
fn test_app_panel_accessors_and_switching() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left.clone())?;
    app.right_panel = Panel::new(right.clone())?;

    assert_eq!(app.get_active_panel_ref().path, left);
    assert_eq!(app.get_inactive_panel_ref().path, right);

    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    assert_eq!(app.get_active_panel_ref().path, right);

    app.swap_panels();
    assert_eq!(app.left_panel.path, right);
    assert_eq!(app.right_panel.path, left);

    Ok(())
}

#[test]
fn test_app_get_drives_and_all_drives_are_non_empty() {
    let drives = App::get_drives();
    assert!(!drives.is_empty());

    let all = App::get_all_drives();
    assert!(!all.is_empty());
    assert!(matches!(all[0], DriveEntry::Local(_)));
}

#[test]
fn test_app_active_panel_mut_accessors() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;

    let mut app = App::new()?;
    app.left_panel = Panel::new(left.clone())?;
    app.right_panel = Panel::new(right.clone())?;

    app.get_active_panel().path = left.join("child");
    assert_eq!(app.left_panel.path, left.join("child"));

    app.get_inactive_panel().path = right.join("other");
    assert_eq!(app.right_panel.path, right.join("other"));

    Ok(())
}
