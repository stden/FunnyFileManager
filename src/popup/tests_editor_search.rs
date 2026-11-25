//! Unit tests: F7 in the editor — the dialog, the search it describes, and
//! Shift+F7 repeating it.

use super::*;
use crate::models::Editor;
use crate::search::{Case, Direction, SearchField, WordMatch};
use std::path::PathBuf;

mod tests_guards_and_chords;
mod tests_not_found;
mod tests_options_and_repeat;

fn editing(content: &str) -> App {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "notes.txt".to_owned(),
        PathBuf::from("notes.txt"),
        content,
    )));
    app
}

fn press(app: &mut App, code: KeyCode) {
    process_popup_input(app, KeyEvent::new(code, KeyModifiers::empty()));
}

fn press_shift(app: &mut App, code: KeyCode) {
    process_popup_input(app, KeyEvent::new(code, KeyModifiers::SHIFT));
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

fn editor(app: &App) -> &Editor {
    app.get_editor().expect("editor is open")
}

fn caret(app: &App) -> (usize, usize) {
    let editor = editor(app);
    (editor.cursor_line, editor.cursor_col)
}

#[test]
fn f7_opens_the_search_dialog() {
    let mut app = editing("alpha\nbeta");
    assert!(editor(&app).search.prompt.is_none(), "closed to start with");

    press(&mut app, KeyCode::F(7));

    assert!(editor(&app).search.prompt.is_some(), "F7 opens the dialog");
}

/// While the dialog is open it owns the keyboard: a typed character goes into
/// the query, not into the file.
#[test]
fn typing_fills_the_query_rather_than_the_buffer() {
    let mut app = editing("alpha\nbeta");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "beta");

    let editor = editor(&app);
    assert_eq!(editor.search.prompt.as_ref().unwrap().query, "beta");
    assert_eq!(editor.lines, vec!["alpha", "beta"], "the file is untouched");
}

#[test]
fn enter_runs_the_search_and_moves_the_caret() {
    let mut app = editing("alpha\nbeta\ngamma");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "gamma");
    press(&mut app, KeyCode::Enter);

    assert!(editor(&app).search.prompt.is_none(), "the dialog closes");
    assert_eq!(caret(&app), (2, 0));
}

#[test]
fn backspace_edits_the_query() {
    let mut app = editing("alpha");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "alphx");
    press(&mut app, KeyCode::Backspace);
    type_text(&mut app, "a");
    press(&mut app, KeyCode::Enter);

    assert_eq!(caret(&app), (0, 0));
}

#[test]
fn escape_closes_the_dialog_without_searching() {
    let mut app = editing("alpha\nbeta");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "beta");
    press(&mut app, KeyCode::Esc);

    let editor = editor(&app);
    assert!(editor.search.prompt.is_none(), "the dialog closes");
    assert!(editor.search.last.is_none(), "nothing was searched for");
    assert_eq!(caret(&app), (0, 0), "the caret did not move");
}

/// Tab walks the controls and Space flips whichever checkbox it lands on, so
/// the options are reachable without the mouse Far never had.
#[test]
fn tab_walks_the_controls_and_space_toggles_a_checkbox() {
    let mut app = editing("text");
    press(&mut app, KeyCode::F(7));

    press(&mut app, KeyCode::Tab);
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().field,
        SearchField::Case
    );
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().case,
        Case::Match
    );

    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().words,
        WordMatch::WholeWords
    );

    // Direction is not a checkbox: Far gives it two buttons, so Tab lands on
    // `Find previous` and Enter is what commits to going backwards.
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().field,
        SearchField::FindPrevious
    );
}
