//! Purpose: the search helpers are inert without an editor or a dialog, and the keys and chords the search dialog has no use for change nothing.

use super::*;
use crate::popup::editor_search::run_search;

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
