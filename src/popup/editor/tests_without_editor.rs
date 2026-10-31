//! Purpose: Editor input and commands that arrive with no editor open do nothing,
//! and the wheel moves the cursor one line, the way the keyboard does.

use std::fs;

use tempfile::TempDir;

use super::save::{confirm_overwrite, save_editor};
use super::*;
use crate::models::{AfterSave, Confirm, EditState, Editor, ExitChoice, OverwritePrompt, Panel};
use crate::mouse::Wheel;
use crate::popup::{process_popup_input, tests::app_with_editor};

fn closed_app() -> App {
    let root = TempDir::new().expect("temporary directory");
    App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("panel"),
        Panel::new(root.path().to_path_buf()).expect("panel"),
    )
}

fn editor(app: &App) -> &Editor {
    app.get_editor().expect("the editor is open")
}

#[test]
fn a_key_with_no_editor_open_is_not_consumed() {
    let mut app = closed_app();

    assert!(!process_editor_input(
        &mut app,
        KeyCode::Char('a'),
        KeyModifiers::NONE
    ));
}

#[test]
fn a_command_with_no_editor_open_does_nothing() {
    let mut app = closed_app();

    apply_editor_command(&mut app, EditorCommand::Copy);

    assert!(app.get_editor().is_none());
}

#[test]
fn saving_with_no_editor_open_writes_nothing() {
    let mut app = closed_app();

    save_editor(&mut app);

    assert!(app.get_editor().is_none());
}

#[test]
fn an_overwrite_answer_with_no_editor_open_is_ignored() {
    let mut app = closed_app();
    let prompt = OverwritePrompt {
        then: AfterSave::Stay,
        answer: Confirm::No,
    };

    confirm_overwrite(&mut app, KeyCode::Char('y'), prompt);

    assert!(app.get_editor().is_none());
}

#[test]
fn an_unrelated_key_leaves_the_overwrite_question_open() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    fs::write(&path, "on disk\n").expect("test setup");
    let mut app = app_with_editor("in the buffer\n", path);
    let prompt = OverwritePrompt {
        then: AfterSave::Stay,
        answer: Confirm::No,
    };
    app.get_editor_mut().expect("open").state = EditState::ConfirmingOverwrite(prompt);

    confirm_overwrite(&mut app, KeyCode::Char('x'), prompt);

    assert_eq!(editor(&app).state, EditState::ConfirmingOverwrite(prompt));
}

#[test]
fn an_exit_answer_with_no_editor_open_is_ignored() {
    let mut app = closed_app();

    confirm_exit(&mut app, KeyCode::Enter, ExitChoice::default());

    assert!(app.get_editor().is_none());
}

#[test]
fn closing_with_no_editor_open_is_ignored() {
    let mut app = closed_app();

    close_editor(&mut app);

    assert!(app.get_editor().is_none());
}

#[test]
fn the_wheel_moves_the_cursor_up_a_line() {
    let temp = TempDir::new().expect("temporary directory");
    let path = temp.path().join("text.txt");
    fs::write(&path, "one\ntwo\n").expect("test setup");
    let mut app = app_with_editor("one\ntwo\n", path);
    assert!(process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
    ));
    assert_eq!(editor(&app).cursor_line, 1);

    assert!(wheel(&mut app, Wheel::Up, KeyModifiers::NONE));

    assert_eq!(editor(&app).cursor_line, 0);
}
