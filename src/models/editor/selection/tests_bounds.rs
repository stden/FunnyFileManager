//! Purpose: Selection endpoints set past the buffer through the public cursor
//! and anchor are clamped before a selection edit erases lines.

use std::path::PathBuf;

use super::*;

#[test]
fn stale_selection_endpoints_are_clamped_before_erasing_lines() {
    for reversed in [false, true] {
        let mut editor = Editor::from_content("draft".into(), PathBuf::from("draft"), "abc\ndef");
        let (anchor, active) = if reversed {
            (Caret::new(0, 1), Caret::new(usize::MAX, usize::MAX))
        } else {
            (Caret::new(usize::MAX, usize::MAX), Caret::new(0, 1))
        };
        editor.selection = EditorSelection::Anchored(anchor);
        editor.cursor_line = active.line.get();
        editor.cursor_col = active.column.chars();
        assert!(editor.delete_selection());
        assert_eq!(editor.content(), "a");
        assert_eq!(editor.caret(), Caret::new(0, 1));
    }
}
