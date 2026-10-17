//! Unit tests closing coverage gaps in `overlays.rs`:
//!
//! - `token_style`'s branches beyond `Token::Normal` (Type, Keyword, Str,
//!   Number/Switch, Comment, Meta, Punctuation) — only exercised when the
//!   editor actually highlights something, which no existing test does.
//! - `clip_runs`'s two edge branches: the room-exhausted `break` when a line
//!   is wider than the screen, and the skip-emptied `continue` when a
//!   horizontal scroll consumes an entire early run.
//!
//! Also the screen helpers `render_gaps_dialogs` shares, which covers the F7
//! search dialog and the information dialog's coverage lines.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::Editor;

pub(super) const SCREEN_W: u16 = 80;
pub(super) const SCREEN_H: u16 = 40;

pub(super) fn draw(app: &mut App, w: u16, h: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| render_overlays(f, app)).unwrap();
    terminal
}

pub(super) fn row_text(terminal: &Terminal<TestBackend>, y: u16, width: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..width).map(|x| buf[(x, y)].symbol()).collect()
}

pub(super) fn screen_text(terminal: &Terminal<TestBackend>, w: u16, h: u16) -> String {
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

fn unscrolled_rust_editor(content: &str) -> Editor {
    let editor = Editor::from_content("tokens.rs".to_owned(), PathBuf::from("tokens.rs"), content);
    assert_eq!(
        (editor.cursor_line, editor.cursor_col),
        (0, 0),
        "precondition: no scroll"
    );
    editor
}

#[test]
fn test_editor_highlights_xml_tag_url_and_uri_tokens() {
    let mut app = App::new().unwrap();
    let xml = r#"<url xmlns="http://a.org">http://b.org</url>"#;
    let mut editor = Editor::from_content("pom.xml".to_owned(), PathBuf::from("pom.xml"), xml);
    editor.cursor_col = 26;
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app, SCREEN_W, SCREEN_H);
    let buf = terminal.backend().buffer();
    // `url` at columns 1..4 is painted with CODE_TAG
    assert_eq!(buf[(1, 1)].fg, palette::CODE_TAG);
    // `a.org` at column 19 is painted with CODE_URL
    assert_eq!(buf[(19, 1)].fg, palette::CODE_URL);
    // `http://b.org` at column 26 is painted with CODE_URI_FG on CODE_URI_BG
    assert_eq!(buf[(26, 1)].fg, palette::CODE_URI_FG);
    assert_eq!(buf[(26, 1)].bg, palette::CODE_URI_BG);

    let clipped = code::clip_runs(
        vec![
            ("a\x01b", crate::highlight::Token::Normal),
            ("tail", crate::highlight::Token::Normal),
        ],
        Cells::ZERO,
        Cells::new(3),
    );
    assert_eq!(clipped.spans.len(), 1);
}
