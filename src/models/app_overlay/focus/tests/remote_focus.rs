//! Remote dialogs expose only the appropriate ordinary text field.

use super::{app, paste};
use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteInput, RemoteLogin};
use crate::models::remote_work::RemoteTarget;
use crate::models::{
    Confirm, HostName, Overlay, PanelSide, Port, Protocol, RemoteOverlay, Session,
};
use tempfile::TempDir;

fn login() -> RemoteLogin {
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    );
    RemoteLogin::new(session)
}

#[test]
fn remote_user_field_accepts_plain_text_but_password_and_confirmation_do_not() {
    let root = TempDir::new().expect("temp");
    let mut app = app(
        &root,
        Overlay::Remote(Box::new(RemoteOverlay(RemoteDialog::Login(login())))),
    );
    paste(&mut app, "alice");
    let Overlay::Remote(dialog) = &mut app.overlay else {
        panic!("remote")
    };
    let RemoteDialog::Login(login_prompt) = &mut dialog.0 else {
        panic!("login")
    };
    assert_eq!(login_prompt.user.text(), "alice");
    login_prompt.field = LoginField::Password;
    assert!(app.focused_text_field().is_none());
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Trust {
            login: login(),
            fingerprint: "SHA256:example".to_owned(),
            answer: Confirm::No,
        },
    ))));
    assert!(app.focused_text_field().is_none());
}

#[test]
fn remote_mask_input_accepts_paste_until_cancel_is_focused() {
    let root = TempDir::new().expect("temp");
    let target = RemoteTarget::Connect {
        side: PanelSide::Left,
        session: login().session.name,
    };
    let dialog = RemoteDialog::Input {
        target,
        purpose: RemoteInput::Selection { select: true },
        input: LineEditor::new("*.rs"),
        cancel: false,
        notice: String::new(),
    };
    let mut app = app(&root, Overlay::Remote(Box::new(RemoteOverlay(dialog))));
    paste(&mut app, ",*.txt");
    let Overlay::Remote(dialog) = &mut app.overlay else {
        panic!("remote")
    };
    let RemoteDialog::Input { input, cancel, .. } = &mut dialog.0 else {
        panic!("input")
    };
    assert_eq!(input.text(), "*.rs,*.txt");
    *cancel = true;
    assert!(app.focused_text_field().is_none());
}
