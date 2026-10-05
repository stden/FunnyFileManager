//! Unit tests: the editor's title bar marks a modified buffer with a star,
//! and the body scrolls sideways to keep the cursor's column visible.

use std::path::PathBuf;

use super::editor_exit::{
    app_on_empty_dirs, body_row, cells, draw, editor_area, editor_with_numbered_body, row_text,
    SCREEN_W,
};
use super::*;
use crate::models::Editor;

// -----------------------------------------------------------------------
// Modified marker in the editor title
// -----------------------------------------------------------------------

#[test]
fn test_editor_title_marks_a_modified_buffer_with_a_star() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.insert_char('x'); // the only supported way to set `modified`
    assert!(editor.state.is_modified(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let area = editor_area();

    // Far's editor bar: the file on the left, the cursor position on the right.
    let bar = row_text(&terminal, area.y);
    assert!(bar.starts_with(" notes.txt *"), "editor bar:\n{bar}");
    assert!(bar.trim_end().ends_with("Ch 0x0020"), "editor bar:\n{bar}");
}

#[test]
fn test_editor_title_has_no_star_for_an_unmodified_buffer() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let editor = editor_with_numbered_body("notes.txt");
    assert!(!editor.state.is_modified(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let area = editor_area();

    // A blank is read past the name: an appearing marker would fill it, so this
    // pins the absence of " *" and not just its prefix.
    let bar = row_text(&terminal, area.y);
    assert!(bar.starts_with(" notes.txt  "), "editor bar:\n{bar}");
}

// -----------------------------------------------------------------------
// Horizontal scrolling: the cursor must stay inside the visible window
// -----------------------------------------------------------------------

#[test]
fn test_editor_scrolls_horizontally_so_the_cursor_column_stays_visible() {
    /// Editor inner width: the full screen, since the editor draws no border.
    const INNER_W: u16 = SCREEN_W;
    const LINE_LEN: usize = 200;

    let (mut app, _left, _right) = app_on_empty_dirs();
    let line = alphabet_line(LINE_LEN);
    let mut editor = Editor::from_content("wide.txt".to_owned(), PathBuf::from("wide.txt"), &line);
    editor.move_end();
    assert_eq!(editor.cursor_col, LINE_LEN, "precondition: cursor at EOL");
    app.set_overlay(Overlay::Editor(editor));

    let mut terminal = draw(&mut app);
    let area = editor_area();

    // The cursor sits one past the last char, on the rightmost inner column,
    // so the window starts LINE_LEN - (INNER_W - 1) chars into the line.
    let first_visible = LINE_LEN - (usize::from(INNER_W) - 1);
    let expected: String = line.chars().skip(first_visible).collect();
    assert_eq!(
        cells(&terminal, body_row(0), area.x, INNER_W - 1),
        expected,
        "visible window must end at the cursor, got row:\n{}",
        row_text(&terminal, body_row(0))
    );
    // …and the head of the line has scrolled out of view.
    assert!(
        !expected.starts_with(&line.chars().take(3).collect::<String>()),
        "the line must not still start at the left edge"
    );

    // The terminal cursor follows the same offset instead of running off the
    // right edge of the overlay.
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(area.x + (INNER_W - 1), body_row(0)),
        "terminal cursor position"
    );
}

fn alphabet_line(length: usize) -> String {
    "abcdefghijklmnopqrstuvwxyz"
        .chars()
        .cycle()
        .take(length)
        .collect()
}
