//! Password paste reads the injected clipboard and never copies password text.

use crate::clipboard::ClipboardBackend;
use crate::models::remote_dialog::{LoginField, RemoteDialog, RemoteLogin};
use crate::models::{App, HostName, Overlay, Panel, Port, Protocol, RemoteOverlay, Session};
use crate::popup::{paste_text, process_popup_input};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Clipboard {
    text: String,
    writes: Rc<Cell<usize>>,
}

impl ClipboardBackend for Clipboard {
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.text.clone())
    }
    fn set_native(&mut self, _text: &str) -> Result<(), String> {
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }
    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }
}

fn app() -> (tempfile::TempDir, App, Rc<Cell<usize>>) {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let session = Session::new(
        Protocol::Sftp,
        HostName::new("example.test").unwrap(),
        Port::SSH,
        None,
    );
    let mut login = RemoteLogin::new(session);
    login.field = LoginField::Password;
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Login(login),
    ))));
    let writes = Rc::new(Cell::new(0));
    app.clipboard.install(Box::new(Clipboard {
        text: "ab\nc\t".into(),
        writes: Rc::clone(&writes),
    }));
    (dir, app, writes)
}

fn password(app: &App) -> &str {
    let Overlay::Remote(dialog) = &app.overlay else {
        panic!("dialog")
    };
    let RemoteDialog::Login(login) = &dialog.0 else {
        panic!("login")
    };
    &login.credentials.password
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    assert!(process_popup_input(app, KeyEvent::new(code, modifiers)));
}

#[test]
fn password_paste_strips_control_characters_and_copy_never_writes_to_clipboard() {
    let (_dir, mut app, writes) = app();
    press(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(password(&app), "abc");
    press(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(password(&app), "abcabc");
    press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(writes.get(), 0);
    assert_eq!(password(&app), "abcabc");
}

#[test]
fn password_paste_has_a_limit_and_buttons_ignore_paste() {
    let (_dir, mut app, _writes) = app();
    paste_text(&mut app, &"x".repeat(4098));
    assert_eq!(password(&app).len(), 4096);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    paste_text(&mut app, "ignored");
    assert_eq!(password(&app).len(), 4096);
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(
        password(&app).len(),
        4096,
        "button focus never edits the password"
    );
}
