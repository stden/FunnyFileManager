//! Purpose: the F7 dialog's options and buttons, Shift+F7's repeat, and a search that finds nothing.

use super::*;

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
