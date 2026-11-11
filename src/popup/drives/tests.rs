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

mod hotkeys;
mod panel_folders;
mod plugin_rows;
mod selecting;

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
