//! Purpose: unit tests for typing, deleting and moving the caret, Unicode
//! included.

use super::*;

#[test]
fn test_editor_insert_char() {
    let mut editor = editor_from("ac\n");
    editor.move_right();
    editor.insert_char('b');
    assert_eq!(editor.content(), "abc\n");
    assert_eq!(editor.cursor_col, 2);
    assert!(editor.state.is_modified());
}

#[test]
fn test_editor_insert_newline_splits_line() {
    let mut editor = editor_from("abcd\n");
    editor.move_right();
    editor.move_right();
    editor.insert_newline();
    assert_eq!(editor.content(), "ab\ncd\n");
    assert_eq!(editor.cursor_line, 1);
    assert_eq!(editor.cursor_col, 0);
}

#[test]
fn test_editor_backspace_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.move_down(1);
    editor.move_right();
    editor.backspace();
    assert_eq!(editor.content(), "ab\nd\n");

    editor.backspace(); // at col 0 — join with previous line
    assert_eq!(editor.content(), "abd\n");
    assert_eq!(editor.cursor_line, 0);
    assert_eq!(editor.cursor_col, 2);

    let mut at_start = editor_from("x\n");
    at_start.backspace(); // nothing before the first char
    assert_eq!(at_start.content(), "x\n");
    assert!(!at_start.state.is_modified());
}

#[test]
fn test_editor_delete_forward_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "b\ncd\n");

    editor.move_right(); // to end of "b"
    editor.delete_forward(); // join with next line
    assert_eq!(editor.content(), "bcd\n");

    editor.move_down(1);
    editor.cursor_col = editor.lines[0].chars().count();
    editor.delete_forward(); // at end of last line — no-op
    assert_eq!(editor.content(), "bcd\n");
}

#[test]
fn test_editor_unicode_editing() {
    let mut editor = editor_from("файл\n");
    editor.move_right();
    editor.move_right();
    editor.insert_char('ё');
    assert_eq!(editor.content(), "фаёйл\n");
    editor.backspace();
    assert_eq!(editor.content(), "файл\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "фал\n");
}

#[test]
fn test_editor_cursor_movement_and_clamping() {
    let mut editor = editor_from("long line\nab\n");
    editor.cursor_col = 9;
    editor.move_down(1);
    assert_eq!(editor.cursor_col, 2, "column clamps to shorter line");
    editor.move_down(100);
    assert_eq!(editor.cursor_line, 1, "clamped to last line");
    editor.move_up(100);
    assert_eq!(editor.cursor_line, 0);

    // Left from col 0 wraps to end of previous line; right from EOL wraps down.
    editor.move_down(1);
    editor.cursor_col = 0;
    editor.move_left();
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 9));
    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));
}
