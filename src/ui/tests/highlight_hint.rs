//! Purpose: painting the editor hands the edit span to the highlight cache,
//! so a change near the end of a long buffer does not hash the prefix.

use std::path::PathBuf;

use crate::models::Editor;

fn editor_of(lines: usize) -> Editor {
    let body = (0..lines)
        .map(|index| format!("let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    Editor::from_content("notes.rs".to_owned(), PathBuf::from("notes.rs"), &body)
}

fn paint(editor: &Editor, rows: usize) {
    let cells = super::super::cells::Cells::columns;
    drop(super::super::code::editor_window(
        editor,
        0,
        rows,
        cells(0),
        cells(40),
    ));
}

#[test]
fn painting_an_edit_uses_the_dirty_span() {
    let mut editor = editor_of(400);
    paint(&editor, 400);
    editor.cursor_line = 300;
    editor.insert_char('x');
    let before_hashes = crate::highlight::HASH_PASSES.get();
    let before_lines = crate::highlight::LINE_PASSES.get();
    paint(&editor, 400);
    assert_eq!(crate::highlight::LINE_PASSES.get() - before_lines, 1);
    assert!(
        crate::highlight::HASH_PASSES.get() - before_hashes < 20,
        "hashes {}",
        crate::highlight::HASH_PASSES.get() - before_hashes
    );
}
