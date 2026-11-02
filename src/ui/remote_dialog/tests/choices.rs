//! Valid remote dialog inputs become typed requests for the active host.

use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{RemoteDialog, RemoteInput};
use crate::models::remote_work::{FileLocation, RemoteTarget, RemoteTask};
use crate::models::{App, Overlay, PanelKind, PanelSide, RemoteOverlay};
use crate::popup::process_popup_input;
use crossterm::event::KeyCode;

fn host() -> (tempfile::TempDir, App, RemoteTarget, FileLocation) {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let app = crate::test_support::netbox::app_on_host(dir.path(), &[("/old", 3)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    let target = RemoteTarget::Panel {
        side: PanelSide::Left,
        link: panel.link().clone(),
        path: panel.path().clone(),
    };
    let root = FileLocation::Remote {
        link: panel.link().clone(),
        path: panel.path().clone(),
    };
    (dir, app, target, root)
}

fn submit(app: &mut App, target: RemoteTarget, purpose: RemoteInput, text: &str) -> RemoteTask {
    let dialog = RemoteDialog::Input {
        target,
        purpose,
        input: LineEditor::new(text),
        cancel: false,
        notice: String::new(),
    };
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
    assert!(process_popup_input(app, KeyCode::Enter.into()));
    app.remote_requests.take().expect("one queued request").task
}

#[test]
fn rename_input_resolves_a_name_in_the_same_remote_directory() {
    let (_dir, mut app, target, root) = host();
    let old = root.child("old").expect("old name");
    let task = submit(&mut app, target, RemoteInput::Rename(old.clone()), "new");
    let RemoteTask::Rename { from, to } = task else {
        panic!("rename")
    };
    assert_eq!(from, old);
    assert_eq!(to.label(), "/new");
}

#[test]
fn transfer_input_resolves_destination_and_keeps_move_intent() {
    let (dir, mut app, target, root) = host();
    let source = root.child("old").expect("source");
    let purpose = RemoteInput::Transfer {
        sources: vec![source.clone()],
        directory: FileLocation::Local(dir.path().to_path_buf()),
        moving: true,
    };
    let task = submit(&mut app, target, purpose, "destination");
    let RemoteTask::Transfer {
        sources,
        destination,
        moving,
    } = task
    else {
        panic!("transfer")
    };
    assert_eq!(sources, vec![source]);
    assert_eq!(
        destination,
        FileLocation::Local(dir.path().join("destination"))
    );
    assert!(moving);
}

#[test]
fn chmod_input_converts_octal_and_keeps_all_selected_targets() {
    let (_dir, mut app, target, root) = host();
    let source = root.child("old").expect("source");
    let task = submit(
        &mut app,
        target,
        RemoteInput::Chmod(vec![source.clone()]),
        "0640",
    );
    let RemoteTask::Chmod { targets, mode } = task else {
        panic!("chmod")
    };
    assert_eq!(targets, vec![source]);
    assert_eq!(mode.bits(), 0o640);
}
