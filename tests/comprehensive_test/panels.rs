//! Purpose: Panel switching and swapping, directory entries and the drive list.

use super::*;

// ==================== App Methods ====================

#[test]
fn test_switch_panel() {
    let mut app = App::new().unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn test_get_active_inactive_panel() {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    app.active_panel = PanelSide::Left;

    assert_eq!(app.get_active_panel().path, left);
    assert_eq!(app.get_inactive_panel().path, right);

    app.active_panel = PanelSide::Right;
    assert_eq!(app.get_active_panel().path, right);
    assert_eq!(app.get_inactive_panel().path, left);
}

#[test]
fn test_swap_panels_preserves_marks() {
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir(&left).unwrap();
    fs::create_dir(&right).unwrap();
    fs::write(left.join("a.txt"), b"").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left.clone()).unwrap();
    app.right_panel = Panel::new(right.clone()).unwrap();
    app.left_panel.marked.insert(FileName::from("a.txt"));

    app.swap_panels();

    // After swap, marks should move with the panel content
    assert!(app.right_panel.marked.contains("a.txt"));
    assert_eq!(app.left_panel.marked.len(), 0);
    assert_eq!(app.left_panel.path, right);
    assert_eq!(app.right_panel.path, left);
}

// ==================== DriveEntry ====================

#[test]
fn test_drive_entry_display() {
    let local = DriveEntry::Local(PathBuf::from("/home"));
    assert_eq!(local.display(), "/home");

    let s3 = DriveEntry::S3Profile("myprofile".to_owned());
    assert_eq!(s3.display(), "S3: myprofile");
}

#[test]
fn test_get_drives_returns_root() {
    let drives = App::get_drives();
    assert_ne!(drives.len(), 0);
    #[cfg(not(windows))]
    assert!(drives.contains(&PathBuf::from("/")));
}

#[test]
fn test_get_all_drives_includes_local() {
    let all = App::get_all_drives();
    assert_ne!(all.len(), 0);
    // At least one local drive
    assert!(all.iter().any(|d| matches!(d, DriveEntry::Local(_))));
}

// ==================== DirEntry ====================

#[test]
fn test_dir_entry_struct() {
    let entry = DirEntry {
        name: "test".to_owned(),
        kind: EntryKind::Directory,
        size: FileSize::from_bytes(0),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(0));

    let file = DirEntry {
        name: "f.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(42),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };
    assert!(!file.kind.is_dir());
    assert_eq!(file.size, FileSize::from_bytes(42));
}
