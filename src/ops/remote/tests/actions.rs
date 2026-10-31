//! Remote actions operate on the visible remote listing and open the right dialog.

use tempfile::TempDir;

use super::super::{actions, dialogs};
use crate::action::{Action, DialogCommand, FileCommand};
use crate::models::remote_dialog::{RemoteDialog, RemoteInput};
use crate::models::remote_work::FileLocation;
use crate::models::{App, Confirm, Overlay, PanelKind, RowMotion};
use crate::test_support::netbox::{app_in, app_on_host};

const FILES: [(&str, u64); 2] = [("/etc/hostname", 5), ("/etc/hosts", 10)];

fn select_file(app: &mut App) {
    let PanelKind::Remote(panel) = &mut app.left_panel.kind else {
        panic!("remote panel expected");
    };
    panel.move_cursor(RowMotion::Next);
}

#[test]
fn local_actions_are_left_to_the_local_dispatcher() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!actions::handle(&mut app, Action::Copy));
    assert!(!actions::handle(&mut app, Action::View));
}

#[test]
fn parent_row_is_not_a_source_for_remote_file_actions() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
    for action in [
        Action::View,
        Action::Edit,
        Action::Copy,
        Action::Move,
        Action::Rename,
        Action::DeleteConfirm,
    ] {
        assert!(actions::handle(&mut app, action));
        assert!(!app.is_overlay_active(), "parent cannot be a file source");
    }
    assert!(!actions::handle(&mut app, Action::Quit));
}

#[test]
fn remote_copy_and_move_show_the_local_destination_and_remote_source() {
    let dir = TempDir::new().expect("temp dir");
    for (action, expected_moving) in [(Action::Copy, false), (Action::Move, true)] {
        let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
        select_file(&mut app);
        assert!(actions::handle(&mut app, action));
        let Overlay::Remote(overlay) = &app.overlay else {
            panic!("transfer input expected");
        };
        let RemoteDialog::Input {
            purpose:
                RemoteInput::Transfer {
                    sources,
                    directory,
                    moving,
                },
            input,
            ..
        } = &overlay.0
        else {
            panic!("transfer input expected");
        };
        assert_eq!(*moving, expected_moving);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name().expect("file name"), "hostname");
        assert_eq!(*directory, FileLocation::Local(dir.path().to_path_buf()));
        assert_eq!(input.text(), dir.path().display().to_string());
    }
}

#[test]
fn rename_and_delete_target_the_remote_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
    select_file(&mut app);

    assert!(actions::handle(&mut app, Action::Rename));
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("rename input expected")
    };
    let RemoteDialog::Input {
        purpose: RemoteInput::Rename(source),
        input,
        ..
    } = &overlay.0
    else {
        panic!("rename input expected");
    };
    assert_eq!(source.name().expect("file name"), "hostname");
    assert_eq!(input.text(), "hostname");

    assert!(actions::handle(&mut app, Action::DeleteConfirm));
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("delete confirmation expected")
    };
    let RemoteDialog::Delete {
        sources, answer, ..
    } = &overlay.0
    else {
        panic!("delete confirmation expected");
    };
    assert_eq!(*answer, Confirm::Yes);
    assert_eq!(sources[0].name().expect("file name"), "hostname");
}

#[test]
fn mkdir_and_permissions_target_the_remote_panel() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
    select_file(&mut app);

    assert!(actions::handle(
        &mut app,
        Action::Dialog(DialogCommand::Mkdir)
    ));
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("mkdir input expected")
    };
    assert!(
        matches!(&overlay.0, RemoteDialog::Input { purpose: RemoteInput::Mkdir(FileLocation::Remote { path, .. }), .. } if path.as_str() == "/etc")
    );

    assert!(actions::handle(
        &mut app,
        Action::File(FileCommand::Attributes)
    ));
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("permissions input expected")
    };
    assert!(
        matches!(&overlay.0, RemoteDialog::Input { purpose: RemoteInput::Chmod(sources), .. } if sources.len() == 1)
    );
}

#[test]
fn selection_dialog_applies_its_mask_to_the_remote_listing() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &FILES, Some("/etc"));
    assert!(actions::handle(&mut app, Action::SelectGroup));
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("selection input expected");
    };
    let RemoteDialog::Input {
        target,
        purpose,
        input,
        ..
    } = &overlay.0
    else {
        panic!("selection input expected");
    };
    assert_eq!(input.text(), "*");
    let (target, purpose) = (target.clone(), purpose.clone());
    dialogs::input(&mut app, target, purpose, "host*").expect("select mask");
    assert_eq!(marked(&app), ["hostname", "hosts"]);
    assert!(actions::handle(&mut app, Action::InvertSelection));
    assert_eq!(marked(&app), Vec::<String>::new());
    assert!(actions::handle(&mut app, Action::RestoreSelection));
    assert_eq!(marked(&app), ["hostname", "hosts"]);
}

fn marked(app: &App) -> Vec<String> {
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel expected")
    };
    panel.action_names()
}
