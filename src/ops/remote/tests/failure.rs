//! Failed remote requests keep credentials and unsaved edits available for retry.

use super::super::{completion, editor};
use crate::models::remote_dialog::{LoginField, RemoteDialog};
use crate::models::remote_work::{FileLocation, RemoteRequest, RemoteTarget, RemoteTask};
use crate::models::{AfterSave, Confirm, EditState, Fingerprint, Overlay, PanelKind, PanelSide};
use crate::remote::{Credentials, RemoteError};
use crate::test_support::netbox::{app_in, app_on_host, app_on_sessions, file_store, session};

fn connect_request(session: crate::models::Session) -> RemoteRequest {
    let credentials = Credentials {
        user: "alice".to_owned(),
        password: "example-only".to_owned().into(),
        ..Credentials::default()
    };
    RemoteRequest {
        id: 1,
        target: RemoteTarget::Connect {
            side: PanelSide::Left,
            session: session.name.clone(),
        },
        task: RemoteTask::Connect {
            session,
            credentials,
        },
    }
}

#[test]
fn unknown_host_key_reopens_confirmation_with_login_and_password_intact() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let session = session("example.test", None);
    let store = file_store(dir.path(), vec![session.clone()]);
    let mut app = app_on_sessions(dir.path(), store);
    let PanelKind::Sessions(panel) = &mut app.left_panel.kind else {
        panic!("sessions panel")
    };
    panel.focus(&session.name);
    let error = RemoteError::HostKey("SHA256:test".to_owned());
    completion::apply(&mut app, connect_request(session), Err(error));
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("trust dialog")
    };
    let RemoteDialog::Trust {
        login,
        fingerprint,
        answer,
    } = &dialog.0
    else {
        panic!("trust confirmation")
    };
    assert_eq!(fingerprint, "SHA256:test");
    assert_eq!(*answer, Confirm::No);
    assert_eq!(login.user.text(), "alice");
    assert_eq!(login.credentials.password.as_str(), "example-only");
    assert!(matches!(login.field, LoginField::Password));
}

#[test]
fn authentication_failure_keeps_credentials_and_explains_the_error() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let session = session("example.test", None);
    let store = file_store(dir.path(), vec![session.clone()]);
    let mut app = app_on_sessions(dir.path(), store);
    let PanelKind::Sessions(panel) = &mut app.left_panel.kind else {
        panic!("sessions panel")
    };
    panel.focus(&session.name);
    completion::apply(
        &mut app,
        connect_request(session),
        Err(RemoteError::AuthenticationRequired),
    );
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("login dialog")
    };
    let RemoteDialog::Login(login) = &dialog.0 else {
        panic!("login dialog")
    };
    assert_eq!(login.user.text(), "alice");
    assert_eq!(login.credentials.password.as_str(), "example-only");
    assert_eq!(
        login.notice,
        RemoteError::AuthenticationRequired.to_string()
    );
    assert!(matches!(login.field, LoginField::Password));
}

#[test]
fn late_login_failure_does_not_reopen_a_dialog_for_a_different_panel() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_in(dir.path());
    completion::apply(
        &mut app,
        connect_request(session("example.test", None)),
        Err(RemoteError::AuthenticationRequired),
    );
    assert!(matches!(app.overlay, Overlay::None));
}

fn save_request(target: RemoteTarget, source: FileLocation) -> RemoteRequest {
    RemoteRequest {
        id: 2,
        target,
        task: RemoteTask::Save {
            source,
            bytes: b"saved version".to_vec(),
            expected: Fingerprint::of(b"old"),
            overwrite: false,
            then: AfterSave::Close,
            revision: 0,
        },
    }
}

fn opened_editor() -> (
    tempfile::TempDir,
    crate::models::App,
    RemoteTarget,
    FileLocation,
) {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_on_host(dir.path(), &[("/file.txt", 5)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    let target = RemoteTarget::Panel {
        side: PanelSide::Left,
        link: panel.link().clone(),
        path: panel.path().clone(),
    };
    let source = FileLocation::Remote {
        link: panel.link().clone(),
        path: panel.path().join("file.txt").expect("file path"),
    };
    editor::open(&mut app, source.clone(), b"unsaved contents", true).expect("editor");
    (dir, app, target, source)
}

#[test]
fn conflicting_save_retains_buffer_and_asks_before_overwriting() {
    let (_dir, mut app, target, source) = opened_editor();
    completion::apply(
        &mut app,
        save_request(target, source),
        Err(RemoteError::Conflict),
    );
    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(editor.content(), "unsaved contents");
    let EditState::ConfirmingOverwrite(prompt) = editor.state else {
        panic!("overwrite prompt")
    };
    assert_eq!(prompt.answer, Confirm::No);
    assert_eq!(prompt.then, AfterSave::Close);
    assert!(editor
        .notice
        .as_deref()
        .expect("notice")
        .contains("Not saved"));
}

#[test]
fn failed_save_keeps_the_buffer_modified_and_shows_the_error() {
    let (_dir, mut app, target, source) = opened_editor();
    completion::apply(
        &mut app,
        save_request(target, source),
        Err(RemoteError::Failed("permission denied")),
    );
    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.content(), "unsaved contents");
    assert!(editor
        .notice
        .as_deref()
        .expect("notice")
        .contains("permission denied"));
}
