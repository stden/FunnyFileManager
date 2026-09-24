//! Unit tests closing coverage gaps in `overlays.rs`:
//!
//! - `token_style`'s branches beyond `Token::Normal` (Type, Keyword, Str,
//!   Number/Switch, Comment, Meta, Punctuation) — only exercised when the
//!   editor actually highlights something, which no existing test does.
//! - `clip_runs`'s two edge branches: the room-exhausted `break` when a line
//!   is wider than the screen, and the skip-emptied `continue` when a
//!   horizontal scroll consumes an entire early run.
//! - `render_search_prompt` (F7's dialog inside the editor) and the cursor
//!   placement that follows it — every existing F7 test drives it through
//!   `popup::process_popup_input` and never renders it.
//! - `render_information`'s `Coverage::Partial` line, never built by a test
//!   that also renders the overlay.

use super::*;

use crate::models::Editor;
use crate::search::{Case, Direction, SearchField, SearchPrompt, WordMatch};
use crate::stats::{Coverage, DirectoryStats};

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use std::path::PathBuf;

const SCREEN_W: u16 = 80;
const SCREEN_H: u16 = 40;

fn draw(app: &mut App, w: u16, h: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| render_overlays(f, app)).unwrap();
    terminal
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16, width: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..width).map(|x| buf[(x, y)].symbol()).collect()
}

fn screen_text(terminal: &Terminal<TestBackend>, w: u16, h: u16) -> String {
    (0..h)
        .map(|y| row_text(terminal, y, w))
        .collect::<Vec<_>>()
        .join("\n")
}

// -----------------------------------------------------------------------
// token_style: the highlighted-token branches, plus clip_runs's room-
// exhausted `break` (a line wider than the screen, no horizontal scroll).
// -----------------------------------------------------------------------

#[test]
fn test_editor_highlights_rust_keywords_types_strings_comments_and_numbers() {
    let mut app = App::new().unwrap();

    // Short enough that neither line is clipped: they exist to hit
    // `token_style`'s Keyword/Type/Str/Comment/Punctuation and Number arms.
    let line0 = "let x: String = \"hi\"; // note";
    let line1 = "let n = 42;";
    // Longer than the 80-column screen and built from several runs, so the
    // renderer's per-run budget reaches zero *before* every run is consumed —
    // the only way `clip_runs` takes its `room.is_zero()` early exit.
    let line2 = format!("let {} {}", "a".repeat(90), "b".repeat(10));

    let content = format!("{line0}\n{line1}\n{line2}");
    let editor = unscrolled_rust_editor(&content);
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app, SCREEN_W, SCREEN_H);
    // Editor bar occupies row 0; source lines start at row 1.
    let row0 = row_text(&terminal, 1, SCREEN_W);
    let row1 = row_text(&terminal, 2, SCREEN_W);
    let row2 = row_text(&terminal, 3, SCREEN_W);

    assert!(
        row0.starts_with(line0),
        "keyword/type/string/comment line must render unclipped, got:\n{row0}"
    );
    assert!(
        row1.starts_with(line1),
        "number line must render unclipped, got:\n{row1}"
    );
    // The line is 105 cells wide on an 80-column screen: it must be cut
    // somewhere in the run of `a`s, and the trailing run of `b`s — which sits
    // in a later run than where the budget ran out — must never appear.
    assert!(
        row2.contains('a') && !row2.contains('b'),
        "long line must be clipped before its final run, got:\n{row2}"
    );
}

#[test]
fn test_editor_highlights_python_decorator_as_meta() {
    let mut app = App::new().unwrap();

    let editor = Editor::from_content("deco.py".to_owned(), PathBuf::from("deco.py"), "@decorator");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app, SCREEN_W, SCREEN_H);
    let row = row_text(&terminal, 1, SCREEN_W);
    assert!(
        row.starts_with("@decorator"),
        "a line opening with the meta prefix must render as one Meta run, got:\n{row}"
    );
}

// -----------------------------------------------------------------------
// clip_runs: the skip-emptied `continue`, reached only when a horizontal
// scroll consumes an entire early run before reaching a later one.
// -----------------------------------------------------------------------

#[test]
fn test_editor_horizontal_scroll_drops_an_entire_leading_run() {
    let mut app = App::new().unwrap();

    // Two short runs ("let", " ") followed by a long one: scrolling the
    // cursor to the end of the line pushes the visible window past both short
    // runs entirely, so `clip_runs` must skip them down to nothing and move
    // on to the third run instead of drawing an empty span for either.
    let tail = "z".repeat(100);
    let content = format!("let {tail}");
    let mut editor = Editor::from_content("wide.rs".to_owned(), PathBuf::from("wide.rs"), &content);
    editor.move_end();
    assert_eq!(
        editor.cursor_col,
        content.chars().count(),
        "precondition: cursor at EOL"
    );
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app, SCREEN_W, SCREEN_H);
    let row = row_text(&terminal, 1, SCREEN_W);

    // The scrolled-past "let " must be gone, and the row must still be full
    // of the visible tail of `z`s rather than blank space.
    assert!(
        !row.contains("let "),
        "scrolled-out run must not reappear, got:\n{row}"
    );
    assert!(
        row.contains('z'),
        "the run scrolled into view must still render, got:\n{row}"
    );
}

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

fn unscrolled_rust_editor(content: &str) -> Editor {
    let editor = Editor::from_content("tokens.rs".to_owned(), PathBuf::from("tokens.rs"), content);
    assert_eq!(
        (editor.cursor_line, editor.cursor_col),
        (0, 0),
        "precondition: no scroll"
    );
    editor
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
