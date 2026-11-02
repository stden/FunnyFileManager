//! Purpose: Edge-case tests for single-line completion fast path, suffix overwrite, and multi-edit caret math.

use super::*;
use crate::lsp::features::TextEdit;

#[test]
fn complete_lsp_replace_overwrites_identifier_suffix_for_fallback_and_explicit_edits() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get_old_name()\n");
    ed.cursor_col = 4;
    let fallback = completion(json!({"label":"get_value"}));
    ed.complete_lsp_replace(query(&ed), &fallback).unwrap();
    assert_eq!(ed.lines[0], "get_value()");
    assert_eq!(ed.cursor_col, 9);

    let mut ed2 = editor(&root, "call(foo_tail, 1)\n");
    ed2.cursor_col = 8;
    let explicit = completion(json!({
        "label": "foobar",
        "textEdit": {
            "range": {"start": {"line": 0, "character": 5}, "end": {"line": 0, "character": 8}},
            "newText": "foobar"
        }
    }));
    ed2.complete_lsp_replace(query(&ed2), &explicit).unwrap();
    assert_eq!(ed2.lines[0], "call(foobar, 1)");
    assert_eq!(ed2.cursor_col, 11);
}

#[test]
fn single_line_completion_rejects_reversed_range_without_mutating_buffer() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "alpha_beta\n");
    ed.cursor_col = 5;
    let mut bad = completion(json!({"label": "alpha"}));
    bad.edit = Some(TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 6,
            },
            end: Position {
                line: 0,
                character: 2,
            },
        },
        new_text: "oops".into(),
    });
    assert_eq!(
        ed.complete_lsp(query(&ed), &bad),
        Err("Invalid completion range")
    );
    assert_eq!(ed.lines[0], "alpha_beta");
    assert!(!ed.state.is_modified());
}

#[test]
fn bare_cr_line_falls_back_cleanly_to_multi_line_preparer() {
    let root = TempDir::new().unwrap();
    let mut cr_ed = editor(&root, "fo\n");
    cr_ed.lines = vec!["fo\r".into()];
    cr_ed.cursor_col = 2;
    let item = completion(json!({"label": "foobar"}));
    cr_ed.complete_lsp(query(&cr_ed), &item).unwrap();
    assert_eq!(cr_ed.lines[0], "foobar\r");
}

#[test]
fn multi_edit_adjusts_caret_only_for_edits_preceding_primary_range() {
    let root = TempDir::new().unwrap();
    let mut multi = editor(&root, "header\ncall(fo)\ntrailer\n");
    multi.cursor_line = 1;
    multi.cursor_col = 7;
    let with_around = completion(json!({
        "label": "foo_bar",
        "textEdit": {
            "range": {"start": {"line": 1, "character": 5}, "end": {"line": 1, "character": 7}},
            "newText": "foo_bar"
        },
        "additionalTextEdits": [
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}}, "newText": "use x;\n"},
            {"range": {"start": {"line": 2, "character": 7}, "end": {"line": 2, "character": 7}}, "newText": " // end"}
        ]
    }));
    multi.complete_lsp(query(&multi), &with_around).unwrap();
    assert_eq!(
        multi.lsp_text(),
        "use x;\nheader\ncall(foo_bar)\ntrailer // end\n"
    );
    assert_eq!((multi.cursor_line, multi.cursor_col), (2, 12));
}

#[test]
fn zero_width_insertions_at_identical_offset_are_rejected_as_overlapping() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "abc\n");
    ed.cursor_col = 0;
    let clashing = completion(json!({
        "label": "a",
        "textEdit": {
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
            "newText": "first"
        },
        "additionalTextEdits": [
            {"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}}, "newText": "second"}
        ]
    }));
    assert_eq!(
        ed.complete_lsp(query(&ed), &clashing),
        Err("Overlapping completion edits")
    );
    assert_eq!(ed.lines[0], "abc");
}
