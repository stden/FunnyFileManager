//! Unit tests: overlay paths no other test drew — the editor's F7 search
//! dialog and its cursor, and the information dialog's coverage lines.

use std::path::PathBuf;

use super::render_gaps_editor::{draw, screen_text, SCREEN_H, SCREEN_W};
use super::*;
use crate::models::Editor;
use crate::search::{Case, Direction, SearchField, SearchPrompt, WordMatch};
use crate::stats::{Coverage, DirectoryStats};

// -----------------------------------------------------------------------
// render_search_prompt (F7 inside the editor) and the cursor it places —
// every popup test drives the dialog through key input and never renders it.
// -----------------------------------------------------------------------

#[test]
fn test_editor_search_prompt_overlay_renders_its_dialog() {
    let mut app = App::new().unwrap();

    let editor = editor_with_search_prompt();
    app.set_overlay(Overlay::Editor(editor));

    let mut terminal = draw(&mut app, SCREEN_W, SCREEN_H);
    let screen = screen_text(&terminal, SCREEN_W, SCREEN_H);

    for text in [
        "Search",
        "Search for",
        "zzzneedle",
        "Case sensitive",
        "Whole words",
        "Find previous",
        "Find next",
        "Cancel",
    ] {
        assert!(screen.contains(text), "missing {text}:\n{screen}");
    }

    // The Query field is focused, so the dialog must have placed the caret —
    // exercising the `Some((x, y))` arm at the call site in `overlays.rs`.
    assert!(
        terminal.get_cursor_position().is_ok(),
        "focused Input row must place the terminal cursor"
    );
}

// -----------------------------------------------------------------------
// render_information: the Coverage::Partial line, which needs stats built
// and rendered together.
// -----------------------------------------------------------------------

#[test]
fn test_information_overlay_reports_partial_coverage() {
    let mut app = App::new().unwrap();

    let stats = DirectoryStats {
        coverage: Coverage::Partial,
        ..Default::default()
    };
    app.set_overlay(Overlay::Information(Box::new(stats)));

    let terminal = draw(&mut app, SCREEN_W, 24);
    let screen = screen_text(&terminal, SCREEN_W, 24);

    assert!(
        screen.contains("measured to the walk limit"),
        "partial-coverage caveat must be drawn, got:\n{screen}"
    );
}

#[test]
fn test_information_overlay_omits_partial_caveat_when_complete() {
    let mut app = App::new().unwrap();

    let stats = DirectoryStats {
        coverage: Coverage::Complete,
        ..Default::default()
    };
    app.set_overlay(Overlay::Information(Box::new(stats)));

    let terminal = draw(&mut app, SCREEN_W, 24);
    let screen = screen_text(&terminal, SCREEN_W, 24);

    assert!(
        !screen.contains("measured to the walk limit"),
        "complete coverage must not show the lower-bound caveat, got:\n{screen}"
    );
}

/// An unreadable folder has no totals to show: its zeros read as an empty
/// folder, which is what F3 on a denied folder used to say.
#[test]
fn test_information_overlay_says_an_unreadable_folder_cannot_be_read() {
    let mut app = App::new().unwrap();
    let stats = DirectoryStats {
        coverage: Coverage::Unreadable("Permission denied (os error 13)".to_owned()),
        ..Default::default()
    };
    app.set_overlay(Overlay::Information(Box::new(stats)));

    let screen = screen_text(&draw(&mut app, SCREEN_W, 24), SCREEN_W, 24);

    assert!(
        screen.contains("Cannot read this folder: Permission denied (os error 13)"),
        "{screen}"
    );
    assert!(!screen.contains("0 files"), "{screen}");
}

/// Folders inside that could not be read make every total a lower bound.
#[test]
fn test_information_overlay_marks_totals_with_unread_folders_as_lower_bounds() {
    let mut app = App::new().unwrap();
    let stats = DirectoryStats {
        files: 3,
        coverage: Coverage::Unread(1),
        ..Default::default()
    };
    app.set_overlay(Overlay::Information(Box::new(stats)));

    let screen = screen_text(&draw(&mut app, SCREEN_W, 24), SCREEN_W, 24);

    assert!(screen.contains("in 3 files"), "{screen}");
    assert!(
        screen.contains("1 folder(s) could not be read — totals are lower bounds"),
        "{screen}"
    );
}

fn editor_with_search_prompt() -> Editor {
    let mut editor = Editor::from_content(
        "notes.txt".to_owned(),
        PathBuf::from("notes.txt"),
        "one\ntwo\n",
    );
    editor.search.prompt = Some(SearchPrompt {
        query: "zzzneedle".to_owned(),
        case: Case::Ignore,
        words: WordMatch::Anywhere,
        direction: Direction::Forward,
        field: SearchField::Query,
    });
    editor
}
