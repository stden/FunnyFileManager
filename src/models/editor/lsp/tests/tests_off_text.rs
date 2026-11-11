//! Purpose: Completion input that points off the text changes nothing: a live
//! refresh with the cursor off the text leaves the popup as it was, and an
//! edit naming a missing line or a reversed range leaves the document as it was.

use super::*;

#[test]
fn a_live_refresh_with_the_cursor_off_the_text_leaves_the_popup_alone() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get\n");
    ed.lsp.state = ServerState::Ready;
    ed.cursor_col = 3;
    let first = query(&ed);
    ed.update_live_completion(first, vec![completion(json!({"label": "get_value"}))]);
    assert!(matches!(ed.lsp.popup, Some(Dialog::Completion { .. })));

    let stale = query(&ed);
    ed.cursor_line = 9;
    ed.update_live_completion(stale, Vec::new());
    assert!(
        matches!(ed.lsp.popup, Some(Dialog::Completion { .. })),
        "the popup stays as it was"
    );
}

/// A missing line takes the multi-line path; a reversed range is refused on
/// the in-place path and, for a line that keeps its CR, on the multi-line one.
#[test]
fn invalid_completion_lines_and_reversed_ranges_preserve_the_document() {
    for (text, line, start, end) in [("abc", 9, 0, 0), ("abc", 0, 2, 1), ("ab\r\ncd\n", 0, 2, 1)] {
        let root = TempDir::new().unwrap();
        let mut editor = editor(&root, text);
        let item = completion(json!({
            "label": "replacement",
            "textEdit": {
                "range": {"start": {"line": line, "character": start},
                          "end": {"line": line, "character": end}},
                "newText": "changed"
            }
        }));
        assert!(editor.complete_lsp(query(&editor), &item).is_err());
        assert_eq!(editor.content(), text);
        assert_eq!(editor.lsp.revision, Revision(0));
    }
}
