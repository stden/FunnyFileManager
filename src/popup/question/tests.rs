//! Unit tests: the yes/no question moves between its answers, answers by
//! letter or by Enter, and acts only on Yes.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, Confirm, Overlay, Question, QuestionPurpose};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::test_support::netbox::{app_in, name, session};

fn asking_to_delete(dir: &TempDir) -> App {
    let mut app = app_in(dir.path());
    app.sessions.upsert(session("a.test", None));
    app.set_overlay(Overlay::Question(Question::new(
        QuestionPurpose::DeleteSession(name("a.test")),
    )));
    app
}

fn answer(app: &App) -> Confirm {
    let Overlay::Question(question) = &app.overlay else {
        panic!("the question");
    };
    question.answer()
}

#[test]
fn yes_by_letter_or_by_enter_acts_and_closes() {
    let dir = TempDir::new().expect("temp dir");
    for yes in [KeyCode::Char('y'), KeyCode::Char('Y'), KeyCode::Enter] {
        let mut app = asking_to_delete(&dir);
        assert!(process_popup_input(&mut app, key(yes)));
        assert!(!app.is_overlay_active(), "{yes:?}");
        assert_eq!(app.sessions.len(), 0, "{yes:?}");
    }
}

#[test]
fn no_by_letter_escape_f10_or_enter_on_no_does_nothing() {
    let dir = TempDir::new().expect("temp dir");
    for no in [
        KeyCode::Char('n'),
        KeyCode::Char('N'),
        KeyCode::Esc,
        KeyCode::F(10),
    ] {
        let mut app = asking_to_delete(&dir);
        process_popup_input(&mut app, key(no));
        assert!(!app.is_overlay_active(), "{no:?}");
        assert_eq!(app.sessions.len(), 1, "{no:?}");
    }
    let mut app = asking_to_delete(&dir);
    process_popup_input(&mut app, key(KeyCode::Right));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(app.sessions.len(), 1);
}

#[test]
fn every_move_key_switches_the_answer_and_others_are_swallowed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = asking_to_delete(&dir);
    let mut expected = Confirm::Yes;
    for code in [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Tab,
        KeyCode::BackTab,
    ] {
        process_popup_input(&mut app, key(code));
        expected = expected.toggled();
        assert_eq!(answer(&app), expected, "{code:?}");
    }
    assert!(process_popup_input(&mut app, key(KeyCode::Char('q'))));
    assert_eq!(answer(&app), expected);
}

/// Asked about another dialog's key, this one says it is not its own.
#[test]
fn another_overlay_is_not_this_dialogs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE
    ));
    assert!(!app.is_overlay_active());
}
