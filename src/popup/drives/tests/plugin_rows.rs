//! Purpose: the drive menu's plugin rows — `NetBox` sessions and the process list — and a local drive turning a `NetBox` panel back into files.

use super::*;

/// The `NetBox` row turns the dialog's own side into the sessions panel; the
/// Process list row opens the Task list, which the closing menu must not
/// take down with it.
#[test]
fn the_netbox_row_opens_the_sessions_panel_on_its_side() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = crate::test_support::netbox::app_in(temp.path());
    let status = select_drive(&mut app, PanelSide::Right, DriveEntry::NetBox);
    assert!(app.right_panel.kind.is_netbox());
    assert!(!app.left_panel.kind.is_netbox());
    assert_eq!(status.to_string(), "NetBox: 0 stored sessions");
}

/// The row opens the one task list Ctrl+W opens, on the installed host: no
/// `ps` runs, and closing the drive menu does not take the viewer down.
#[test]
fn the_process_list_row_opens_the_task_list_and_the_menu_stays_closed() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = crate::test_support::netbox::app_in(temp.path());
    let host = crate::test_support::processes::FixtureHost::new(Vec::new());
    app.task_host.install(std::sync::Arc::new(host));
    open_drive_menu(&mut app, 0);
    let Overlay::DriveSelection { rows, .. } = &app.overlay else {
        panic!("the drive menu is open");
    };
    let digit = rows
        .iter()
        .find(|row| row.entry == DriveEntry::ProcessList)
        .and_then(|row| row.key)
        .expect("the process list has a digit");
    process_popup_input(&mut app, key(KeyCode::Char(digit)));
    assert!(
        matches!(app.overlay, Overlay::Tasks(_)),
        "the task list is open"
    );
}

/// Without a host the row falls back to the report, as Ctrl+W does there.
#[cfg(unix)]
#[test]
fn without_a_host_the_process_list_row_opens_the_report() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = crate::test_support::netbox::app_in(temp.path());
    let status = select_drive(&mut app, PanelSide::Left, DriveEntry::ProcessList);
    assert!(matches!(status, Status::Cleared));
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report is open");
    };
    assert_eq!(viewer.title(), "Task list");
}

/// A local drive picked for a panel that shows the sessions list or a host
/// puts the panel back on files first, even when the drive cannot be read.
#[test]
fn a_local_drive_turns_a_netbox_panel_back_into_files() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("panel"),
        Panel::new(temp.path().to_path_buf()).expect("panel"),
    );
    crate::test_support::netbox::open_empty_sessions(&mut app, PanelSide::Left);
    let status = select_drive(
        &mut app,
        PanelSide::Left,
        DriveEntry::Local(temp.path().to_path_buf()),
    );
    assert!(matches!(status, Status::ChangedTo(_)), "{status:?}");
    assert_eq!(app.left_panel.kind, crate::models::PanelKind::Files);

    crate::test_support::netbox::open_empty_sessions(&mut app, PanelSide::Left);
    let status = select_drive(
        &mut app,
        PanelSide::Left,
        DriveEntry::Local(temp.path().join("missing")),
    );
    assert!(
        matches!(status, Status::OpenDriveFailed { .. }),
        "{status:?}"
    );
    assert_eq!(app.left_panel.kind, crate::models::PanelKind::Files);
}
