//! Unit tests: the editor and the viewer scroll past `u16::MAX` lines without
//! wrapping, and the editor places its cursor in cells.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::render_widths::{blank_app, screen_text};
use super::*;
use crate::models::Editor;
use crate::scroll::ScrollTop;

/// Horizontal editor scrolling past u16 must not wrap around: a minified
/// one-line file is well under the edit size limit and easily 200k chars wide.
#[test]
fn test_editor_scrolls_past_u16_without_wrapping() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let content = format!("{}MARKER", "x".repeat(70_000));
    let mut editor =
        Editor::from_content("bundle.min.js".to_owned(), PathBuf::from("b.js"), &content);
    editor.move_end();
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let screen = screen_text(&terminal);
    assert!(
        screen.contains("MARKER"),
        "editor must show the text under the cursor, got:\n{screen}"
    );
}

/// The editor overlay is the third site of the chars-vs-cells rule: the model's
/// `cursor_col` is a char index (right for editing), but the horizontal window
/// and the cursor land on screen COLUMNS. On a CJK line a char-indexed window
/// scrolls half as far as it should and the block cursor sits under the wrong
/// glyph.
#[test]
fn test_editor_overlay_scrolls_and_places_the_cursor_in_cells() {
    let backend = TestBackend::new(30, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    // 40 CJK chars = 80 cells, far wider than the ~25-cell editor window.
    let content = format!("{}标记", "文".repeat(40));
    let mut editor = Editor::from_content("cjk.txt".to_owned(), PathBuf::from("cjk.txt"), &content);
    editor.move_end();
    app.set_overlay(Overlay::Editor(editor));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // A wide glyph occupies two buffer cells and the second renders blank, so
    // assert on the glyphs rather than on an exact substring.
    let screen = screen_text(&terminal);
    assert!(
        screen.contains('标') && screen.contains('记'),
        "the window must scroll far enough to show the text under the cursor, got:\n{screen}"
    );

    // The cursor sits one cell past "记", at the last column: budgeting in chars
    // would have halved the scroll and parked it mid-line instead. The editor
    // is borderless and full-screen, so "last column" is the screen's.
    //
    // One cell short of the right edge, not on it: the window starts on an odd
    // cell, and a double-width glyph straddling that cut is dropped whole
    // rather than half-drawn — so the row ends one column early and the cursor
    // goes with it.
    let area = terminal.backend().buffer().area;
    assert_eq!(
        terminal.get_cursor_position().unwrap().x,
        area.x + area.width - 2,
        "the cursor must follow the double-width text, not its char count"
    );
}

/// Same wraparound for the viewer's vertical offset.
#[test]
fn test_viewer_scrolls_past_u16_without_wrapping() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let mut lines: Vec<String> = (0..70_000).map(|i| format!("line {i}")).collect();
    lines[69_000] = "MARKER".to_owned();
    app.set_overlay(Overlay::Viewer(Viewer {
        scroll: ScrollTop::TOP.down(69_000, usize::MAX),
        ..Viewer::report("huge.log", lines)
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let screen = screen_text(&terminal);
    assert!(
        screen.contains("MARKER"),
        "viewer must show the scrolled-to line, got:\n{screen}"
    );
}
