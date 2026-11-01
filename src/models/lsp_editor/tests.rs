//! Purpose: Pin the dialog state: the selection, the draft's changes, Del's
//! two meanings, what a save does to the table, and which problems block F2.

use serde_json::{json, Value};

use super::*;
use crate::config::Enablement;

/// A writable dialog over the catalog with `file`'s `lsp` object laid on it.
fn editor(file: &Value) -> LspEditor {
    let loaded: LspSettings = serde_json::from_value(file.clone()).expect("a valid lsp object");
    LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        loaded,
    )
}

fn language(key: &str) -> LanguageId {
    LanguageId::from_key(key)
}

fn selected(editor: &LspEditor) -> &str {
    editor.selected().expect("a selection").as_str()
}

mod form_tests;

mod tests_form_edges;
mod tests_notice;
mod tests_paging_edges;
mod tests_problems;
mod tests_selection;
mod tests_view;
