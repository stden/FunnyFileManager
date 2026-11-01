//! Keyboard and paste behavior of remote modal dialogs.

use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteInput, RemoteLogin};
use crate::models::remote_work::{FileLocation, RemoteTarget};
use crate::models::{
    App, Confirm, HostName, Overlay, Panel, PanelSide, Port, Protocol, RemoteOverlay, Session,
};
use crate::popup::{paste_text, process_popup_input};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn session() -> Session {
    Session::new(
        Protocol::Sftp,
        HostName::new("example.test").expect("host"),
        Port::SSH,
        None,
    )
}

pub(super) fn target() -> RemoteTarget {
    RemoteTarget::Connect {
        side: PanelSide::Left,
        session: session().name,
    }
}

fn app_with(dialog: RemoteDialog) -> App {
    let directory = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(directory.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(dialog))));
    app
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    assert!(process_popup_input(app, KeyEvent::new(code, modifiers)));
}

fn login(app: &App) -> &RemoteLogin {
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("remote dialog")
    };
    let RemoteDialog::Login(login) = &dialog.0 else {
        panic!("login dialog")
    };
    login
}

#[test]
fn login_navigation_and_paste_keep_password_out_of_the_user_field() {
    let mut app = app_with(RemoteDialog::Login(RemoteLogin::new(session())));
    paste_text(&mut app, "alice\nsmith\n");
    assert_eq!(login(&app).user.text(), "alice smith");
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert!(matches!(login(&app).field, LoginField::Password));
    paste_text(&mut app, "abc\nxyz");
    assert_eq!(login(&app).credentials.password.as_str(), "abcxyz");
    press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(login(&app).credentials.password.as_str(), "abcxy");
    press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert!(login(&app).credentials.password.is_empty());
    press(&mut app, KeyCode::Char('z'), KeyModifiers::NONE);
    assert_eq!(login(&app).credentials.password.as_str(), "z");
    press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert!(matches!(login(&app).field, LoginField::User));
    assert_eq!(login(&app).user.text(), "alice smith");
}

#[test]
fn cancel_login_does_not_queue_a_connection() {
    let mut login = RemoteLogin::new(session());
    login.field = LoginField::Cancel;
    let mut app = app_with(RemoteDialog::Login(login));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.remote_requests.is_idle());
}

#[test]
fn question_toggle_and_rejection_close_without_queuing() {
    let mut app = app_with(RemoteDialog::Delete {
        target: target(),
        sources: vec![FileLocation::Local("important.txt".into())],
        answer: Confirm::No,
    });
    press(&mut app, KeyCode::Right, KeyModifiers::NONE);
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("remote dialog")
    };
    let RemoteDialog::Delete { answer, .. } = &dialog.0 else {
        panic!("delete dialog")
    };
    assert!(answer.is_yes());
    press(&mut app, KeyCode::Char('n'), KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.remote_requests.is_idle());
}

#[test]
fn invalid_input_is_kept_for_correction_and_cancel_closes_it() {
    let mut app = app_with(RemoteDialog::Input {
        target: target(),
        purpose: RemoteInput::Mkdir(FileLocation::Local("directory".into())),
        input: LineEditor::new("draft"),
        cancel: false,
        notice: String::new(),
    });
    paste_text(&mut app, "\nmore");
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("remote dialog")
    };
    let RemoteDialog::Input { input, notice, .. } = &dialog.0 else {
        panic!("input dialog")
    };
    assert_eq!(input.text(), "draftmore");
    assert!(notice.contains("remote panel has changed"));
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(app.remote_requests.is_idle());
}
