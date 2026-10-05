//! Purpose: opening the chosen drive — Enter on the highlighted row, a vanished folder, an unreadable drive, and an S3 profile.

use super::*;

/// Enter applies the highlighted drive to the panel the menu was opened for and
/// closes the menu. Exercises the whole wiring: list lookup, panel move, status.
///
/// A drive remembers its current directory, as in Far: a panel already on the
/// chosen drive lands back in the folder it was in, not at the root.
#[test]
fn drive_menu_enter_opens_the_highlighted_drive() {
    let temp = TempDir::new().expect("temp dir");

    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    open_drive_menu(&mut app, 0);

    assert!(process_popup_input(&mut app, key(KeyCode::Enter)));

    let first = App::get_all_drives()
        .into_iter()
        .next()
        .expect("at least one drive");
    let DriveEntry::Local(root) = first else {
        panic!("the first drive is expected to be a local one");
    };
    let expected = if temp.path().starts_with(&root) {
        temp.path().to_path_buf()
    } else {
        root
    };
    assert_eq!(app.left_panel.path, expected);
    assert_eq!(
        app.status_text(),
        format!("Changed to {}", expected.display())
    );
    assert!(
        !app.is_overlay_active(),
        "the menu closes once a drive is chosen"
    );
}

/// The drive's "current folder" is the panel's, and that folder may have been
/// deleted: the drive then opens at the nearest folder above it that exists,
/// instead of failing with "No such file or directory".
#[test]
fn a_drive_whose_current_folder_was_deleted_opens_at_the_nearest_existing_one() {
    let temp = TempDir::new().expect("temp dir");
    let inner = temp.path().join("gone").join("inner");
    std::fs::create_dir_all(&inner).expect("mkdir");
    let left = Panel::new(inner).expect("panel");
    let right = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(left, right);
    std::fs::remove_dir_all(temp.path().join("gone")).expect("rm");
    let root = temp
        .path()
        .ancestors()
        .last()
        .expect("absolute")
        .to_path_buf();

    let status = select_drive(&mut app, PanelSide::Left, DriveEntry::Local(root));

    assert_eq!(status, Status::ChangedTo(temp.path().to_path_buf()));
    assert_eq!(app.left_panel.path, temp.path());
}

/// Picking a readable local drive reloads the panel there and reports the path.
#[test]
fn selecting_a_local_drive_moves_the_panel_and_reports_the_new_path() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("sub")).expect("mkdir");

    let mut app = App::new().expect("test app");
    // "sub" also exists in the destination, so only an explicit clear_marks()
    // drops it — the reload's stale-mark filter alone would keep it.
    app.right_panel.marked.insert(FileName::from("sub"));

    let status = select_drive(
        &mut app,
        PanelSide::Right,
        DriveEntry::Local(temp.path().to_path_buf()),
    );

    assert_eq!(status, Status::ChangedTo(temp.path().to_path_buf()));
    assert_eq!(app.right_panel.path, temp.path());
    assert!(
        app.right_panel.entries.iter().any(|e| e.name == "sub"),
        "the destination listing was loaded"
    );
    assert!(app.right_panel.marked.is_empty(), "marks were dropped");
    assert!(
        app.right_panel.selected().is_some(),
        "a non-empty panel keeps a selection"
    );
}

/// A drive that cannot be read (removable media that is gone, an unmounted
/// letter) must surface the io error instead of silently reporting success —
/// and leave the panel on the directory it was showing.
///
/// This used to assert the opposite ("the panel follows the drive even when the
/// listing fails"), pinning the bug: the panel ended up on a path with no
/// listing, marks already dropped, and the stale selection index pointing past
/// the one remaining "..".
#[test]
fn selecting_an_unreadable_drive_reports_the_io_error_and_stays_put() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("keep.txt"), b"x").expect("write");
    let missing = temp.path().join("no-such-drive");
    let expected = std::fs::read_dir(&missing)
        .expect_err("the path must not exist")
        .to_string();

    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.left_panel.marked.insert(FileName::from("keep.txt"));

    let status = select_drive(&mut app, PanelSide::Left, DriveEntry::Local(missing));

    assert_eq!(status, Status::OpenDriveFailed { error: expected });
    assert_eq!(
        app.left_panel.path,
        temp.path(),
        "a panel must never show a path it has no listing for"
    );
    assert!(
        app.left_panel.entries.iter().any(|e| e.name == "keep.txt"),
        "the previous listing is still there"
    );
    assert!(
        app.left_panel.marked.contains("keep.txt"),
        "marks belong to the directory we never left"
    );
    assert!(
        app.left_panel
            .selected()
            .is_some_and(|index| index < app.left_panel.entries.len()),
        "the selection must stay inside the listing"
    );
}

/// S3 browsing is not implemented: picking a profile only acknowledges it and
/// must leave the local panel — path and marks — exactly as it was.
#[test]
fn selecting_an_s3_profile_acknowledges_it_without_touching_the_panel() {
    let mut app = App::new().expect("test app");
    let before = app.left_panel.path.clone();
    app.left_panel.marked.insert(FileName::from("keep.txt"));

    let status = select_drive(
        &mut app,
        PanelSide::Left,
        DriveEntry::S3Profile("prod".to_owned()),
    );

    assert_eq!(status, Status::S3ProfileSelected("prod".to_owned()));
    assert_eq!(app.left_panel.path, before);
    assert!(
        app.left_panel.marked.contains("keep.txt"),
        "an S3 profile must not disturb the local panel"
    );
}
