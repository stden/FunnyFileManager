//! Purpose: Pin the edit form: tab order, what OK accepts and refuses, the
//! values it writes into the draft, and the two states of a suggestion list.

use serde_json::json;

use super::super::*;
use crate::config::{patch::OwnedField, Enablement, ValueError};
use crate::models::{Candidate, CandidateKind, Menu};

fn editor() -> LspEditor {
    LspEditor::new(
        Access::Writable {
            path: "/srv/x/settings.yaml".into(),
        },
        LspSettings::default(),
    )
}

fn form(editor: &mut LspEditor) -> &mut ServerForm {
    editor.form_mut().expect("the form is open")
}

/// Replace the text of `field`, as typing would.
fn type_into(editor: &mut LspEditor, field: FormField, text: &str) {
    let form = form(editor);
    form.focus_on(field);
    let line = form.focused_line_mut().expect("a text field");
    line.set_text(text);
}

fn menu(texts: &[&str]) -> Menu {
    Menu::new(
        texts
            .iter()
            .map(|text| Candidate {
                text: (*text).to_owned(),
                kind: CandidateKind::Word,
                hint: None,
            })
            .collect(),
    )
    .expect("candidates")
}

mod tests_ok;
mod tests_suggestions;
mod tests_tab_order;
