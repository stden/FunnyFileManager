//! Purpose: A cursor past the last line is moved onto it before text is typed,
//! so the text lands on a real line rather than nowhere.

use std::path::PathBuf;

use crate::models::Editor;

#[test]
fn a_cursor_past_the_last_line_types_onto_the_last_line() {
    let mut editor = Editor::from_content("t.txt".into(), PathBuf::from("t.txt"), "abc");
    editor.cursor_line = 5;
    editor.insert_text("xyz");
    assert_eq!(editor.lines, vec!["xyzabc".to_owned()]);
}
