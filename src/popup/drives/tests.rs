//! Drive menu (Alt+F1 / Alt+F2): index movement, opening the picked drive,
//! and the Windows-only quick-select-by-letter key.

use super::super::process_popup_input;
use super::super::tests::key;
use super::select_drive;
use crate::models::FileName;
use std::path::PathBuf;

use crate::models::{App, DriveEntry, Overlay, Panel, PanelSide};
use crate::status::Status;

use crossterm::event::KeyCode;

use tempfile::TempDir;

/// Open the drive menu on the left panel the way Alt+F1 does: with the rows
/// scanned once, at open, and stored on the overlay.
fn open_drive_menu(app: &mut App, selected_index: usize) {
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index,
        rows: crate::models::drive_rows(),
    });
}

/// The menu's current selection — every test using this keeps the menu open.
fn drive_index(app: &App) -> usize {
    if let Overlay::DriveSelection { selected_index, .. } = app.overlay {
        selected_index
    } else {
        panic!("drive menu is no longer open");
    }
}

/// Unix reports a single drive ("/"), so the decrement branch is only reached
/// from a higher index on a multi-drive system (Windows). The arm is pure index
/// arithmetic and never consults the drive list, so starting the menu at index 3
/// runs exactly the code a Windows drive list would.
#[test]
fn drive_menu_up_moves_to_the_previous_drive() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 3);

    assert!(process_popup_input(&mut app, key(KeyCode::Up)));
    assert_eq!(drive_index(&app), 2);
}

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
        DriveEntry::S3Profile("prod".to_string()),
    );

    assert_eq!(status, Status::S3ProfileSelected("prod".to_string()));
    assert_eq!(app.left_panel.path, before);
    assert!(
        app.left_panel.marked.contains("keep.txt"),
        "an S3 profile must not disturb the local panel"
    );
}

/// Quick-select by letter opens `C:\`, `D:\`, … — paths that never exist on Unix
/// and need not exist on Windows either. Without the existence guard the panel
/// would jump to an unreadable path and the menu would close; instead the key is
/// swallowed and nothing moves.
#[test]
fn drive_menu_letter_key_does_nothing_when_that_drive_does_not_exist() {
    // `C:\` is real on Windows, so the key pressed has to be a letter this
    // machine has no drive for. On Unix the first candidate always qualifies.
    let absent = ('a'..='z')
        .find(|letter| !PathBuf::from(format!("{}:\\", letter.to_ascii_uppercase())).exists())
        .expect("no machine has all 26 drive letters mounted");

    let temp = TempDir::new().expect("temp dir");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    open_drive_menu(&mut app, 0);
    let status_before = app.status_text();

    assert!(process_popup_input(&mut app, key(KeyCode::Char(absent))));

    assert_eq!(app.left_panel.path, temp.path());
    assert_eq!(app.status_text(), status_before);
    assert_eq!(drive_index(&app), 0, "the menu stays open on its selection");
}

/// Down is pure index arithmetic against the live drive count, so the wrap
/// happens at the *last* index — whatever that is on this build.
///
/// It used to start at 0 and assert Down left it there, which only held when
/// the list had exactly one entry (`/` on Unix). With `--features s3` the S3
/// profiles pad the list and the same Down legitimately advanced, so the test
/// failed on a build it was never written for. Starting at the end pins the
/// wrap itself.
#[test]
fn drive_menu_down_wraps_at_the_end_of_the_list() {
    let last = App::get_all_drives().len().saturating_sub(1);
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, last);

    assert!(process_popup_input(&mut app, key(KeyCode::Down)));
    assert_eq!(
        drive_index(&app),
        0,
        "past the last drive, wrap to the first"
    );
}

/// When more than one drive is visible, Down advances the highlight. S3
/// profiles pad the list on builds with the `s3` feature; without them this
/// is skipped rather than failing a single-drive Unix host.
#[test]
fn drive_menu_down_advances_when_there_are_several_drives() {
    let drives = App::get_all_drives();
    if drives.len() < 2 {
        return;
    }
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);

    assert!(process_popup_input(&mut app, key(KeyCode::Down)));
    assert_eq!(
        drive_index(&app),
        1,
        "Down from the first drive must select the second"
    );
}

/// Letter quick-select opens the drive when that path exists (Windows). Pins
/// the `open_drive` success path that Unix never reaches because `C:\` is not
/// a real path there.
#[cfg(windows)]
#[test]
fn drive_menu_letter_opens_an_existing_local_drive() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);

    // System drive is almost always C: on Windows CI images.
    assert!(process_popup_input(&mut app, key(KeyCode::Char('c'))));
    assert!(
        !app.is_overlay_active(),
        "choosing a real drive letter closes the menu"
    );
    assert!(
        app.left_panel.path.to_string_lossy().starts_with("C:"),
        "panel should sit on the chosen drive, got {}",
        app.left_panel.path.display()
    );
}

#[test]
fn drive_index_steps_wrap_in_both_directions() {
    // Up: decrement, wrap from the top, tolerate an empty list.
    assert_eq!(super::drive_index_up(2, 3), 1);
    assert_eq!(super::drive_index_up(0, 3), 2);
    assert_eq!(super::drive_index_up(0, 0), 0);
    // Down: increment, wrap past the end, tolerate an empty list.
    assert_eq!(super::drive_index_down(0, 3), 1);
    assert_eq!(super::drive_index_down(2, 3), 0);
    assert_eq!(super::drive_index_down(0, 0), 0);
}

/// A key no row answers to is swallowed. This used to build `Q:\` and report
/// that the drive could not be opened, on a machine with no drive letters at
/// all.
#[test]
fn a_key_no_row_answers_to_leaves_the_menu_open() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);
    let before = app.left_panel.path.clone();

    process_popup_input(&mut app, key(KeyCode::Char('Q')));

    assert_eq!(app.left_panel.path, before);
    assert!(
        matches!(app.overlay, Overlay::DriveSelection { .. }),
        "an unknown key must be a harmless keypress"
    );
}

/// The row's own key opens it, exactly as Enter on that row would: Far offers
/// the drive letter and the digit beside a plugin row as shortcuts.
#[test]
fn a_rows_own_key_opens_it_without_the_cursor() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);
    let rows = crate::models::drive_rows();
    let first = rows.first().expect("a machine always has one filesystem");
    let shortcut = first.key.expect("a disk row always has a key");

    process_popup_input(&mut app, key(KeyCode::Char(shortcut)));

    assert!(matches!(app.overlay, Overlay::None), "the menu closes");
    assert!(
        app.status_text().contains("Changed to"),
        "{}",
        app.status_text()
    );
}

/// The lookup runs over the rows stored when the menu opened and ignores
/// case. Pinned with an injected row keyed `z`: the live key on the machines
/// the suite runs on is `/`, whose case swap is the identity, so a lookup
/// through the real drive list never leaves the equal-case path.
#[test]
fn the_key_lookup_ignores_case() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: vec![crate::models::DriveRow {
            key: Some('z'),
            kind: crate::models::RowKind::Disk {
                label: "z-drive".to_owned(),
                device: crate::diskfree::DriveKind::Fixed,
                space: None,
            },
            entry: DriveEntry::Local(temp.path().to_path_buf()),
        }],
    });

    assert!(process_popup_input(&mut app, key(KeyCode::Char('Z'))));

    assert!(matches!(app.overlay, Overlay::None), "the menu closes");
    assert_eq!(
        app.left_panel.path,
        temp.path(),
        "`Z` must reach the row keyed `z`"
    );
}

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
