//! Submitting remote dialogs changes the visible panel or queues one operation.

use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{RemoteDialog, RemoteInput};
use crate::models::remote_work::{FileLocation, RemoteTarget, RemoteTask};
use crate::models::{App, Confirm, Overlay, PanelKind, PanelSide, RemoteOverlay};
use crate::popup::process_popup_input;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn remote_target(app: &App) -> (RemoteTarget, FileLocation) {
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    (
        RemoteTarget::Panel {
            side: PanelSide::Left,
            link: panel.link().clone(),
            path: panel.path().clone(),
        },
        FileLocation::Remote {
            link: panel.link().clone(),
            path: panel.path().clone(),
        },
    )
}

fn show(app: &mut App, dialog: RemoteDialog) {
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
}

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

#[test]
fn mkdir_submits_checked_destination_as_one_remote_request() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(dir.path(), &[], None);
    let (target, directory) = remote_target(&app);
    show(
        &mut app,
        RemoteDialog::Input {
            target,
            purpose: RemoteInput::Mkdir(directory),
            input: LineEditor::new("reports"),
            cancel: false,
            notice: String::new(),
        },
    );
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None));
    let request = app.remote_requests.take().expect("queued request");
    let RemoteTask::Mkdir(FileLocation::Remote { path, .. }) = request.task else {
        panic!("queued mkdir")
    };
    assert_eq!(path.as_str(), "/reports");
}

#[test]
fn invalid_mode_stays_open_with_its_error_and_can_be_cancelled() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(dir.path(), &[], None);
    let (target, directory) = remote_target(&app);
    show(
        &mut app,
        RemoteDialog::Input {
            target,
            purpose: RemoteInput::Chmod(vec![directory]),
            input: LineEditor::new("not octal"),
            cancel: false,
            notice: String::new(),
        },
    );
    press(&mut app, KeyCode::Enter);
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("dialog stays open")
    };
    let RemoteDialog::Input { input, notice, .. } = &dialog.0 else {
        panic!("input dialog")
    };
    assert_eq!(input.text(), "not octal");
    assert!(notice.contains("octal"));
    assert!(app.remote_requests.is_idle());
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn selecting_a_mask_changes_remote_marks_without_a_background_job() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(
        dir.path(),
        &[("/notes.txt", 2), ("/code.rs", 3)],
        None,
    );
    let (target, _) = remote_target(&app);
    show(
        &mut app,
        RemoteDialog::Input {
            target,
            purpose: RemoteInput::Selection { select: true },
            input: LineEditor::new("*.txt"),
            cancel: false,
            notice: String::new(),
        },
    );
    press(&mut app, KeyCode::Enter);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    assert!(panel.marked("notes.txt"));
    assert!(!panel.marked("code.rs"));
    assert!(app.remote_requests.is_idle());
}

#[test]
fn confirming_delete_queues_the_exact_selected_sources() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(dir.path(), &[], None);
    let (target, directory) = remote_target(&app);
    let source = directory.child("delete-me").expect("valid child");
    show(
        &mut app,
        RemoteDialog::Delete {
            target,
            sources: vec![source.clone()],
            answer: Confirm::No,
        },
    );
    press(&mut app, KeyCode::Char('y'));
    let request = app.remote_requests.take().expect("queued request");
    let RemoteTask::Delete(sources) = request.task else {
        panic!("queued deletion")
    };
    assert_eq!(sources, vec![source]);
}
