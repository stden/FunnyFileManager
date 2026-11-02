//! Remote dialog mouse controls focus the clicked action before dispatch.

use super::*;
use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteInput, RemoteLogin};
use crate::models::remote_work::{FileLocation, RemoteTarget, RemoteTask};
use crate::models::{HostName, Port, Protocol, RemoteOverlay, Session};

fn login() -> RemoteLogin {
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").unwrap(),
        Port::SSH,
        None,
    );
    RemoteLogin::new(session)
}

fn show(app: &mut App, dialog: RemoteDialog) {
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
}

fn remote(app: &App) -> &RemoteDialog {
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("remote dialog")
    };
    &dialog.0
}

fn target() -> RemoteTarget {
    RemoteTarget::Connect {
        side: PanelSide::Left,
        session: login().session.name,
    }
}

#[test]
fn login_rows_focus_the_clicked_field_and_ignore_captions() {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    show(&mut app, RemoteDialog::Login(login()));
    click_dialog_row(&mut app, 4);
    assert!(
        matches!(remote(&app), RemoteDialog::Login(login) if login.field == LoginField::Password)
    );
    click_dialog_row(&mut app, 0);
    assert!(
        matches!(remote(&app), RemoteDialog::Login(login) if login.field == LoginField::Password)
    );
    click_dialog_row(&mut app, 2);
    assert!(matches!(remote(&app), RemoteDialog::Login(login) if login.field == LoginField::User));
}

#[test]
fn clicking_connect_overrides_cancel_focus_and_queues_the_login() {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    let mut login = login();
    login.user.set_text("alice");
    login.field = LoginField::Cancel;
    show(&mut app, RemoteDialog::Login(login));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(
        matches!(remote(&app), RemoteDialog::Login(login) if login.field == LoginField::Connect)
    );
    assert!(popup::process_popup_input(&mut app, KeyCode::Enter.into()));
    let request = app.remote_requests.take().expect("connection request");
    let RemoteTask::Connect { credentials, .. } = request.task else {
        panic!("connect")
    };
    assert_eq!(credentials.user, "alice");
}

#[test]
fn trust_buttons_set_answer_before_queuing_fingerprint_or_closing() {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    show(
        &mut app,
        RemoteDialog::Trust {
            login: login(),
            fingerprint: "SHA256:example".into(),
            answer: Confirm::No,
        },
    );
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert!(matches!(
        remote(&app),
        RemoteDialog::Trust {
            answer: Confirm::Yes,
            ..
        }
    ));
    assert!(popup::process_popup_input(&mut app, KeyCode::Enter.into()));
    let request = app.remote_requests.take().expect("connection request");
    let RemoteTask::Connect { credentials, .. } = request.task else {
        panic!("connect")
    };
    assert_eq!(
        credentials.trusted_host_key.as_deref(),
        Some("SHA256:example")
    );
}

#[test]
fn delete_cancel_button_selects_no_and_closes_without_a_job() {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    show(
        &mut app,
        RemoteDialog::Delete {
            target: target(),
            sources: vec![],
            answer: Confirm::Yes,
        },
    );
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(
        remote(&app),
        RemoteDialog::Delete {
            answer: Confirm::No,
            ..
        }
    ));
    assert!(popup::process_popup_input(&mut app, KeyCode::Enter.into()));
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.remote_requests.is_idle());
}

#[test]
fn input_row_refocuses_text_and_cancel_button_refocuses_cancel() {
    let temp = TempDir::new().unwrap();
    let mut app = app_at(&temp);
    show(
        &mut app,
        RemoteDialog::Input {
            target: target(),
            purpose: RemoteInput::Mkdir(FileLocation::Local(temp.path().into())),
            input: LineEditor::new("new"),
            cancel: true,
            notice: String::new(),
        },
    );
    click_dialog_row(&mut app, 1);
    assert!(matches!(
        remote(&app),
        RemoteDialog::Input { cancel: false, .. }
    ));
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert!(matches!(
        remote(&app),
        RemoteDialog::Input { cancel: true, .. }
    ));
    assert!(popup::process_popup_input(&mut app, KeyCode::Enter.into()));
    assert!(matches!(app.overlay, Overlay::None));
}
