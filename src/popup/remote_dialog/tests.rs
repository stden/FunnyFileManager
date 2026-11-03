//! Purpose: Remote dialog guards when closed and typing in the login user field.

use super::*;
use crate::models::remote_dialog::{LoginField, RemoteLogin};
use crate::models::{HostName, Panel, Port, Protocol, RemoteOverlay, Session};
use std::path::PathBuf;

#[test]
fn inactive_guards_and_login_user_typing_behave_safely() {
    let empty = || Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    paste(&mut app, "ignored");
    submit::run(&mut app);
    assert!(matches!(app.overlay, Overlay::None));

    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    );
    let mut login = RemoteLogin::new(session);
    login.field = LoginField::User;
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Login(login),
    ))));
    assert!(process(&mut app, KeyCode::Char('u'), KeyModifiers::NONE));
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("remote dialog");
    };
    let RemoteDialog::Login(login) = &dialog.0 else {
        panic!("login");
    };
    assert_eq!(login.user.text(), "u");
}
