//! Purpose: A cursor past the last line is moved onto it before text is typed,
//! so the text lands on a real line rather than nowhere.

use std::path::PathBuf;

use crate::models::{Editor, EditorSelection};

#[test]
fn a_cursor_past_the_last_line_types_onto_the_last_line() {
    let mut editor = Editor::from_content("t.txt".into(), PathBuf::from("t.txt"), "abc");
    editor.cursor_line = 5;
    editor.insert_text("xyz");
    assert_eq!(editor.lines, vec!["xyzabc".to_owned()]);
}

#[test]
fn select_all_paste_normalizes_carriage_returns_and_preserves_trailing_newline() {
    let mut lf_editor = Editor::from_content("t.txt".into(), PathBuf::from("t.txt"), "old\n");
    lf_editor.selection = EditorSelection::All;
    lf_editor.insert_text("alpha\nbeta\n");
    assert_eq!(lf_editor.lines, vec!["alpha".to_owned(), "beta".to_owned()]);
    assert!(lf_editor.format.trailing_newline);

    let mut crlf_editor = Editor::from_content("t.txt".into(), PathBuf::from("t.txt"), "old\n");
    crlf_editor.selection = EditorSelection::All;
    crlf_editor.insert_text("one\r\ntwo\rthree\r\n");
    assert_eq!(
        crlf_editor.lines,
        vec!["one".to_owned(), "two".to_owned(), "three".to_owned()]
    );
    assert!(crlf_editor.format.trailing_newline);
}
