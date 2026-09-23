//! Purpose: Verify editor revision isolation and atomic Unicode completion behavior.

use super::*;
use crate::lsp::{features, positions::Range};
use serde_json::json;
use tempfile::TempDir;

fn editor(root: &TempDir, text: &str) -> Editor {
    Editor::from_content("source".into(), root.path().join("source.rs"), text)
}

fn query(editor: &Editor) -> Query {
    Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: LspFeature::Completion,
    }
}

fn completion(value: serde_json::Value) -> Completion {
    let Reply::Completions(mut items) =
        features::parse(LspFeature::Completion, &json!([value])).unwrap()
    else {
        panic!()
    };
    items.remove(0)
}

#[test]
fn completion_replaces_unicode_prefix_and_preserves_file_line_endings() {
    let root = TempDir::new().unwrap();
    let mut editor = editor(&root, "// 😀\r\nпри\r\n");
    editor.cursor_line = 1;
    editor.cursor_col = 3;
    let item = completion(json!({"label":"привет"}));
    editor.complete_lsp(query(&editor), &item).unwrap();
    assert_eq!(editor.content(), "// 😀\r\nпривет\r\n");
    assert_eq!(editor.cursor_col, 6);
    assert_eq!(editor.lsp.revision, Revision(1));
}

#[test]
fn completion_applies_primary_and_additional_edits_atomically() {
    let root = TempDir::new().unwrap();
    let mut editor = editor(&root, "// 😀\nfo");
    editor.cursor_line = 1;
    editor.cursor_col = 2;
    let item = completion(
        json!({"label":"foo", "textEdit":{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":2}},"newText":"foo()"},"additionalTextEdits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"use foo;\n"}]}),
    );
    editor.complete_lsp(query(&editor), &item).unwrap();
    assert_eq!(editor.lsp_text(), "use foo;\n// 😀\nfoo()");
    assert_eq!((editor.cursor_line, editor.cursor_col), (2, 5));
}

#[test]
fn invalid_stale_snippet_and_overlapping_completions_leave_text_untouched() {
    let root = TempDir::new().unwrap();
    let mut editor = editor(&root, "😀foo");
    let mut item = completion(json!({"label":"foo", "insertTextFormat":2,"insertText":"foo($1)"}));
    assert!(editor.complete_lsp(query(&editor), &item).is_err());
    item.snippet = false;
    item.edit = Some(features::TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 1,
            },
            end: Position {
                line: 0,
                character: 2,
            },
        },
        new_text: "x".into(),
    });
    assert!(editor.complete_lsp(query(&editor), &item).is_err());
    item.edit.as_mut().unwrap().range.start.character = 0;
    item.additional_edits.push(item.edit.clone().unwrap());
    assert!(editor.complete_lsp(query(&editor), &item).is_err());
    assert_eq!(editor.lsp_text(), "😀foo");
    let stale = query(&editor);
    editor.insert_char('a');
    assert!(editor.complete_lsp(stale, &item).is_err());
    assert!(!editor.accepts_lsp_reply(stale));
}

#[test]
fn editing_and_cursor_motion_reject_stale_results() {
    let root = TempDir::new().unwrap();
    let mut editor = editor(&root, "fn main() {}");
    let request = query(&editor);
    editor.move_right();
    assert!(!editor.accepts_lsp_reply(request));
    editor.move_left();
    assert!(editor.accepts_lsp_reply(request));
    editor.insert_newline();
    assert!(!editor.accepts_lsp_reply(request));
    let other = Editor::default();
    assert_ne!(other.lsp.id, editor.lsp.id);
}

#[test]
fn snapshots_and_popup_shapes_follow_the_current_editor_context() {
    let root = TempDir::new().unwrap();
    let mut editor = editor(&root, "hello\n");
    assert_eq!(editor.lsp_snapshot().text, "hello\n");
    editor.mark_modified();
    let request = query(&editor);
    for reply in [
        Reply::Completions(Vec::new()),
        Reply::Hover(String::new()),
        Reply::Definitions(Vec::new()),
    ] {
        assert!(editor.show_lsp_reply(request, reply).is_none());
        assert_eq!(editor.notice.as_deref(), Some("No language-server results"));
    }
    let item = Definition {
        path: root.path().join("other.rs"),
        range: Range::default(),
    };
    assert_eq!(
        editor.show_lsp_reply(request, Reply::Definitions(vec![item.clone()])),
        Some(item.clone())
    );
    assert!(editor
        .show_lsp_reply(request, Reply::Definitions(vec![item.clone(), item]))
        .is_none());
    assert!(matches!(editor.lsp.popup, Some(Dialog::Definitions { .. })));
}
