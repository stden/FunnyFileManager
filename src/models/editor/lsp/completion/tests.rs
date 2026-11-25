//! Purpose: Unit tests for `primary_edit` when suffix lookup cannot resolve an end position.

use std::path::PathBuf;

use super::{Completion, Editor, Position, Query, Range};
use crate::lsp::types::{CompletionTrigger, DocumentId, Feature, Revision, Stamp};

fn sample_query(position: Position) -> Query {
    Query {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision(0),
        },
        position,
        feature: Feature::Completion(CompletionTrigger::Manual),
    }
}

fn sample_item(label: &str) -> Completion {
    Completion {
        label: label.to_owned(),
        kind: None,
        detail: String::new(),
        documentation: String::new(),
        insert_text: label.to_owned(),
        edit: None,
        additional_edits: Vec::new(),
        snippet: false,
    }
}

#[test]
fn primary_edit_keeps_query_end_when_cursor_is_past_line_end() {
    let mut editor = Editor::from_content("a.rs".to_owned(), PathBuf::from("a.rs"), "abc");
    editor.cursor_col = 99;
    let pos = Position {
        line: 0,
        character: 3,
    };
    let edit = editor.primary_edit(&sample_item("abcd"), sample_query(pos), true);
    assert_eq!(
        edit.range,
        Range {
            start: pos,
            end: pos
        }
    );
}
