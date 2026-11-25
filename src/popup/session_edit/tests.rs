//! Unit tests: the session dialog's keys — moving, typing, choosing, storing
//! on Enter, keeping the dialog open on a mistake, and closing without
//! storing.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use crate::models::{App, AuthKind, Overlay, Protocol, SessionEdit, SessionField};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::test_support::netbox::{app_in, name};

fn creating(dir: &TempDir) -> App {
    let mut app = app_in(dir.path());
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    app
}

fn dialog(app: &App) -> &SessionEdit {
    let Overlay::SessionEdit(dialog) = &app.overlay else {
        panic!("the session dialog");
    };
    dialog
}

fn press(app: &mut App, codes: &[KeyCode]) {
    for code in codes {
        assert!(process_popup_input(app, key(*code)), "{code:?}");
    }
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, &[KeyCode::Char(ch)]);
    }
}

#[test]
fn tab_and_the_arrows_walk_the_controls() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    press(&mut app, &[KeyCode::Tab, KeyCode::Down]);
    assert_eq!(dialog(&app).field(), SessionField::Host);
    press(&mut app, &[KeyCode::BackTab, KeyCode::Up]);
    assert_eq!(dialog(&app).field(), SessionField::Name);
}

#[test]
fn typing_fills_the_field_and_a_chord_is_not_text() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    type_text(&mut app, "web");
    press(&mut app, &[KeyCode::Backspace]);
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
    );
    assert_eq!(dialog(&app).name, "we");
}

#[test]
fn left_right_and_space_change_a_choice_and_type_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    press(&mut app, &[KeyCode::Tab, KeyCode::Right]);
    assert_eq!(dialog(&app).protocol, Protocol::Scp);
    press(&mut app, &[KeyCode::Char(' ')]);
    assert_eq!(dialog(&app).protocol, Protocol::Ftp);
    press(&mut app, &[KeyCode::Left]);
    assert_eq!(dialog(&app).protocol, Protocol::Scp);
    assert_eq!(dialog(&app).name, "", "a space on a choice is not text");
    for _ in 0..4 {
        press(&mut app, &[KeyCode::Tab]);
    }
    assert_eq!(dialog(&app).field(), SessionField::Auth);
    press(&mut app, &[KeyCode::Right]);
    assert_eq!(dialog(&app).auth, AuthKind::KeyFile);
    press(&mut app, &[KeyCode::Tab, KeyCode::Char(' ')]);
    assert_eq!(
        dialog(&app).key_file,
        " ",
        "a space in a text field is text"
    );
    press(&mut app, &[KeyCode::Left]);
    assert_eq!(
        dialog(&app).auth,
        AuthKind::KeyFile,
        "Left in a text field moves nothing"
    );
}

#[test]
fn enter_stores_the_session_and_closes() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    press(&mut app, &[KeyCode::Tab, KeyCode::Tab]);
    type_text(&mut app, "example.test");
    press(&mut app, &[KeyCode::Enter]);
    assert!(!app.is_overlay_active());
    assert!(app.sessions.find(&name("example.test")).is_some());
}

#[test]
fn a_mistake_keeps_the_dialog_open_with_the_reason() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    press(&mut app, &[KeyCode::Enter]);
    assert!(app.is_overlay_active());
    assert_eq!(app.status_text(), "Error: NetBox: Host: must not be empty");
    assert_eq!(app.sessions.len(), 0);
}

#[test]
fn esc_f10_and_enter_on_cancel_close_without_storing() {
    let dir = TempDir::new().expect("temp dir");
    for closing in [
        vec![KeyCode::Esc],
        vec![KeyCode::F(10)],
        vec![KeyCode::BackTab, KeyCode::Enter],
    ] {
        let mut app = creating(&dir);
        press(&mut app, &[KeyCode::Tab, KeyCode::Tab]);
        type_text(&mut app, "example.test");
        for _ in 0..SessionField::ALL.len() {
            if dialog(&app).field() == SessionField::Name {
                break;
            }
            press(&mut app, &[KeyCode::Tab]);
        }
        assert_eq!(dialog(&app).field(), SessionField::Name);
        press(&mut app, &closing);
        assert!(!app.is_overlay_active(), "{closing:?}");
        assert_eq!(app.sessions.len(), 0, "{closing:?}");
    }
}

#[test]
fn a_key_the_dialog_does_not_use_is_swallowed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = creating(&dir);
    press(&mut app, &[KeyCode::F(5), KeyCode::Insert]);
    assert_eq!(dialog(&app), &SessionEdit::create());
}

/// Asked about another dialog's key, this one says it is not its own.
#[test]
fn another_overlay_is_not_this_dialogs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        KeyModifiers::NONE
    ));
    assert!(!app.is_overlay_active());
}
