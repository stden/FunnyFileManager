//! Unit tests: the input dialog edits its line, moves between its buttons,
//! closes on Esc (unless the dialog settings turn Escape off) and F10, and
//! hands the answer on — or nothing, on Cancel.

use crossterm::event::{KeyCode, KeyModifiers};
use tempfile::TempDir;

use crate::menu::SettingsPage;
use crate::models::{App, InputPrompt, InputPurpose, Overlay};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::test_support::netbox::{app_in, name, session};

fn asking_for_a_copy(dir: &TempDir) -> App {
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("a.test", None));
    app.set_overlay(Overlay::InputPrompt(Box::new(InputPrompt::new(
        InputPurpose::DuplicateSession(name("a.test")),
        "a.test",
    ))));
    app
}

fn typed(app: &App) -> String {
    let Overlay::InputPrompt(prompt) = &app.overlay else {
        panic!("the input prompt");
    };
    prompt.input().text().to_owned()
}

#[test]
fn keys_edit_the_line_and_chords_do_not() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = asking_for_a_copy(&dir);
    for code in [
        KeyCode::Home,
        KeyCode::Char('x'),
        KeyCode::End,
        KeyCode::Backspace,
    ] {
        assert!(process_popup_input(&mut app, key(code)));
    }
    assert_eq!(typed(&app), "xa.tes");
    process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
    );
    assert_eq!(typed(&app), "xa.tes");
}

#[test]
fn enter_on_ok_answers_and_closes_the_dialog_first() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = asking_for_a_copy(&dir);
    process_popup_input(&mut app, key(KeyCode::Char('2')));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.is_overlay_active());
    assert!(app.sessions.find(&name("a.test2")).is_some());
}

#[test]
fn every_move_key_switches_the_button_and_enter_on_cancel_answers_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = asking_for_a_copy(&dir);
    for code in [KeyCode::Tab, KeyCode::BackTab, KeyCode::Up, KeyCode::Down] {
        process_popup_input(&mut app, key(code));
    }
    process_popup_input(&mut app, key(KeyCode::Down));
    let Overlay::InputPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert!(prompt.on_cancel());
    process_popup_input(&mut app, key(KeyCode::Char('2')));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.is_overlay_active());
    assert_eq!(app.sessions.len(), 1, "Cancel copies nothing");
}

#[test]
fn esc_and_f10_close_it_and_escape_can_be_turned_off() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = asking_for_a_copy(&dir);
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(!app.is_overlay_active());

    let mut app = asking_for_a_copy(&dir);
    app.settings
        .set(SettingsPage::Dialog, "off")
        .expect("a known value");
    assert!(!app.settings.enabled(SettingsPage::Dialog));
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert!(app.is_overlay_active(), "Escape is off");
    process_popup_input(&mut app, key(KeyCode::F(10)));
    assert!(!app.is_overlay_active());
    assert_eq!(app.sessions.len(), 1);
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
