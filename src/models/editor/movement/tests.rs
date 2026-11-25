//! Purpose: Unit tests for cursor movement at document boundaries.

use std::path::PathBuf;

use super::Editor;

#[test]
fn move_right_stops_at_end_of_last_line() {
    let mut editor = Editor::from_content("a.txt".to_owned(), PathBuf::from("a.txt"), "ab\nc");
    editor.cursor_line = 0;
    editor.cursor_col = 2;
    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));

    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 1));

    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 1));
}
