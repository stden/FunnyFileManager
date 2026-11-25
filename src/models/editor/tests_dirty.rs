//! Purpose: edits record the changed lines before the caret moves, and a
//! direct write of `lines` leaves the hint empty.

use super::*;

fn editor(content: &str) -> Editor {
    Editor::from_content("notes.rs".to_owned(), PathBuf::from("notes.rs"), content)
}

#[test]
fn typing_records_the_line_and_unions_later_edits() {
    let mut editor = editor("one\ntwo\nthree\nfour\n");
    editor.cursor_line = 3;
    editor.insert_char('x');
    assert_eq!(editor.take_dirty_span(), Some((3, 4)));

    editor.cursor_line = 1;
    editor.insert_char('y');
    editor.cursor_line = 3;
    editor.insert_char('z');
    assert_eq!(editor.take_dirty_span(), Some((1, 4)));
}

#[test]
fn a_newline_and_a_joining_backspace_name_the_line_that_changed() {
    let mut editor = editor("one\ntwo\nthree\n");
    editor.cursor_line = 1;
    editor.cursor_col = 1;
    editor.insert_newline();
    assert_eq!(editor.cursor_line, 2);
    assert_eq!(editor.take_dirty_span(), Some((1, 3)));

    editor.cursor_line = 2;
    editor.cursor_col = 0;
    editor.backspace();
    assert_eq!(editor.cursor_line, 1);
    assert_eq!(editor.take_dirty_span(), Some((1, 2)));
}

#[test]
fn writing_lines_directly_does_not_invent_a_hint() {
    let mut editor = editor("one\ntwo\n");
    editor.lines[0] = "changed".to_owned();
    assert_eq!(editor.take_dirty_span(), None);
}
