//! Unit tests: F7 in the editor — the dialog, the search it describes, and
//! Shift+F7 repeating it.

use super::*;
use crate::models::{Editor, Overlay};
use crate::search::{Case, Direction, SearchField, WordMatch};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

fn editing(content: &str) -> App {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "notes.txt".to_string(),
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

/// The Find buttons choose the direction and run the search in one keystroke.
#[test]
fn the_find_previous_button_searches_backwards() {
    let mut app = editing("match\nfiller\nmatch");
    // Start on the last match so "previous" has somewhere to go.
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "match");
    press(&mut app, KeyCode::Enter);
    assert_eq!(caret(&app), (2, 0));

    press(&mut app, KeyCode::F(7));
    for _ in 0..3 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().field,
        SearchField::FindPrevious
    );
    press(&mut app, KeyCode::Enter);

    assert_eq!(caret(&app), (0, 0), "searched backwards");
    assert_eq!(
        editor(&app).search.last.as_ref().unwrap().direction,
        Direction::Backward,
        "and Shift+F7 will keep going that way"
    );
}

/// Space in the query field is a character, not a toggle — searching for two
/// words has to be possible.
#[test]
fn space_in_the_query_field_is_typed_not_a_toggle() {
    let mut app = editing("hello world");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "hello world");

    let prompt = editor(&app).search.prompt.clone().unwrap();
    assert_eq!(prompt.query, "hello world");
    assert_eq!(prompt.case, Case::Ignore, "no checkbox was flipped");
}

#[test]
fn the_case_option_reaches_the_search() {
    let mut app = editing("Alpha\nalpha");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "alpha");
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' ')); // case sensitive
    press(&mut app, KeyCode::Enter);

    assert_eq!(caret(&app), (1, 0), "the capitalised line must be skipped");
}

/// Shift+F7 repeats without reopening the dialog, walking the matches.
#[test]
fn shift_f7_repeats_the_last_search() {
    let mut app = editing("match\nfiller\nmatch");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "match");
    press(&mut app, KeyCode::Enter);
    assert_eq!(caret(&app), (2, 0), "the first hit is below the caret");

    press_shift(&mut app, KeyCode::F(7));
    assert!(editor(&app).search.prompt.is_none(), "no dialog reopens");
    assert_eq!(caret(&app), (0, 0), "wraps to the other match");
}

/// With nothing to repeat, Shift+F7 asks what to look for rather than doing
/// nothing — which is what the user meant, and saves a trip through F7.
#[test]
fn shift_f7_with_no_previous_search_opens_the_dialog() {
    let mut app = editing("text");
    press_shift(&mut app, KeyCode::F(7));
    assert!(editor(&app).search.prompt.is_some());
}

/// A search that finds nothing has to say so: an unmoved caret is exactly what
/// a key that did not register looks like.
#[test]
fn a_failed_search_is_reported_and_leaves_the_caret_alone() {
    let mut app = editing("alpha\nbeta");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "zeta");
    press(&mut app, KeyCode::Enter);

    assert_eq!(
        editor(&app).search.not_found.as_deref(),
        Some("zeta"),
        "the failure is recorded"
    );
    assert_eq!(caret(&app), (0, 0));
}

/// …and the report clears as soon as the user does anything else.
#[test]
fn the_failure_notice_clears_on_the_next_keystroke() {
    let mut app = editing("alpha");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "zeta");
    press(&mut app, KeyCode::Enter);
    assert!(editor(&app).search.not_found.is_some());

    press(&mut app, KeyCode::Right);
    assert!(editor(&app).search.not_found.is_none());
}

/// Far reopens the dialog on the previous search: repeating it with one option
/// changed is the common case, and retyping the query to get there is the
/// annoying one.
#[test]
fn reopening_the_dialog_keeps_the_previous_query_and_options() {
    let mut app = editing("alpha\nbeta");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "beta");
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' ')); // case sensitive
    press(&mut app, KeyCode::Enter);

    press(&mut app, KeyCode::F(7));
    let prompt = editor(&app).search.prompt.clone().expect("dialog reopened");
    assert_eq!(prompt.query, "beta");
    assert_eq!(prompt.case, Case::Match);
    assert_eq!(
        prompt.field,
        SearchField::Query,
        "the cursor starts on the query"
    );
}

/// An empty query closes the dialog and does nothing, rather than reporting
/// that "" was not found — which is true and useless.
#[test]
fn an_empty_query_searches_for_nothing() {
    let mut app = editing("alpha");
    press(&mut app, KeyCode::F(7));
    press(&mut app, KeyCode::Enter);

    let editor = editor(&app);
    assert!(editor.search.prompt.is_none(), "the dialog closes");
    assert!(editor.search.last.is_none());
    assert!(editor.search.not_found.is_none());
}

/// The Cancel button is Cancel however it is reached.
#[test]
fn enter_on_the_cancel_button_closes_without_searching() {
    let mut app = editing("alpha\nbeta");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "beta");
    for _ in 0..5 {
        press(&mut app, KeyCode::Tab);
    }
    assert_eq!(
        editor(&app).search.prompt.as_ref().unwrap().field,
        SearchField::Cancel
    );
    press(&mut app, KeyCode::Enter);

    assert!(
        editor(&app).search.last.is_none(),
        "nothing was searched for"
    );
    assert_eq!(caret(&app), (0, 0));
}

// ─── the guards, and the keys the search dialog has no use for ──────────────

/// The search helpers are called from a dispatcher that may have closed the
/// editor a moment earlier. Every one of them has to be inert then, rather
/// than reaching into an overlay that is no longer there.
#[test]
fn the_search_helpers_do_nothing_without_an_editor() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::Help);

    assert!(
        !process_search_prompt(&mut app, KeyCode::Enter, KeyModifiers::empty()),
        "no editor, so the prompt handled nothing"
    );
    repeat_search(&mut app);
    run_search(
        &mut app,
        crate::search::Search {
            needle: crate::search::Needle::new("x").expect("non-empty"),
            case: Case::Ignore,
            words: WordMatch::Anywhere,
            direction: Direction::Forward,
        },
    );
    assert!(matches!(app.overlay, Overlay::Help), "still the help page");
}

/// An editor with no dialog open is the other half of the same guard.
#[test]
fn the_search_prompt_handler_does_nothing_while_no_dialog_is_up() {
    let mut app = editing("hello");
    assert!(
        !process_search_prompt(&mut app, KeyCode::Enter, KeyModifiers::empty()),
        "the editor is open but the dialog is not"
    );
}

/// Shift+F7 with nothing to repeat asks what to look for, which is what the
/// user meant and saves a trip through F7.
#[test]
fn repeating_a_search_that_never_happened_opens_the_dialog() {
    let mut app = editing("hello world");
    repeat_search(&mut app);

    let editor = app.get_editor().expect("editor");
    assert!(
        editor.search.prompt.is_some(),
        "it asked instead of doing nothing"
    );
}

#[test]
fn shift_tab_and_up_walk_the_search_controls_backwards() {
    let mut app = editing("hello");
    press(&mut app, KeyCode::F(7));
    let first = search_field(&app);

    press(&mut app, KeyCode::BackTab);
    let back = search_field(&app);
    assert_ne!(back, first);

    press(&mut app, KeyCode::Tab);
    assert_eq!(search_field(&app), first, "and forward again");

    press_shift(&mut app, KeyCode::Up);
    assert_eq!(search_field(&app), back, "Up walks backwards too");
}

/// A key the dialog has no use for is swallowed rather than typed into the
/// query, or every stray function key would corrupt the search.
#[test]
fn a_key_the_search_dialog_has_no_use_for_is_swallowed() {
    let mut app = editing("hello");
    press(&mut app, KeyCode::F(7));
    for ch in "ell".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Insert);

    let editor = app.get_editor().expect("editor");
    let prompt = editor.search.prompt.as_ref().expect("dialog");
    assert_eq!(prompt.query, "ell");
}

fn search_field(app: &App) -> SearchField {
    app.get_editor()
        .expect("editor")
        .search
        .prompt
        .as_ref()
        .expect("dialog")
        .field
}

// ─── the "not found" box is modal ───────────────────────────────────────────

/// Open `content`, search for something it does not hold, and return the app
/// with the "not found" box on screen.
fn after_a_failed_search(content: &str) -> App {
    let mut app = editing(content);
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "zzz");
    press(&mut app, KeyCode::Enter);
    assert!(editor(&app).search.not_found.is_some(), "test setup");
    app
}

/// The box is Far's modal warning: the key that dismisses it is spent on it.
/// Enter used to split the line, Esc closed the editor and a letter was typed
/// into the file, because the key fell through to the editor keymap.
#[test]
fn the_key_that_dismisses_the_not_found_box_never_reaches_the_document() {
    for code in [
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::F(10),
        KeyCode::Char('k'),
    ] {
        let mut app = after_a_failed_search("alpha\nbeta");

        press(&mut app, code);

        let editor = app
            .get_editor()
            .unwrap_or_else(|| panic!("{code:?} closed the editor"));
        assert!(
            editor.search.not_found.is_none(),
            "{code:?} dismissed the box"
        );
        assert_eq!(
            editor.lines,
            ["alpha", "beta"],
            "{code:?} left the text alone"
        );
        assert!(
            !editor.state.is_modified(),
            "{code:?} left the buffer clean"
        );
        assert_eq!(caret(&app), (0, 0), "{code:?} left the caret alone");
    }
}

/// Text arriving while the box is up is no more the document's than a key is:
/// neither a terminal paste nor a batched burst of typing may slip past it.
#[test]
fn a_paste_while_the_not_found_box_is_up_does_not_reach_the_document() {
    let mut app = after_a_failed_search("alpha");

    assert!(
        !editor(&app).accepts_text(),
        "the box owns the input, so typing is not batched into the buffer"
    );
    super::editor::paste_text(&mut app, "pasted");

    assert_eq!(editor(&app).lines, ["alpha"]);
    assert!(!editor(&app).state.is_modified());
}

// ─── chords in the search prompt ────────────────────────────────────────────

/// Crossterm reports Ctrl+A as `Char('a')` with CONTROL, so the letter used to
/// land in the query. Chords are commands, never text, as in every other
/// prompt.
#[test]
fn control_and_alt_chords_do_not_type_into_the_query() {
    let mut app = editing("hello");
    press(&mut app, KeyCode::F(7));
    type_text(&mut app, "zz");

    for (ch, modifiers) in [
        ('a', KeyModifiers::CONTROL),
        ('v', KeyModifiers::CONTROL),
        ('s', KeyModifiers::CONTROL),
        ('x', KeyModifiers::ALT),
    ] {
        process_popup_input(&mut app, KeyEvent::new(KeyCode::Char(ch), modifiers));
    }

    let prompt = editor(&app)
        .search
        .prompt
        .clone()
        .expect("dialog still open");
    assert_eq!(prompt.query, "zz");
    press_shift(&mut app, KeyCode::Char('Q'));
    let prompt = editor(&app)
        .search
        .prompt
        .clone()
        .expect("dialog still open");
    assert_eq!(prompt.query, "zzQ", "Shift is how capitals are typed");
}
