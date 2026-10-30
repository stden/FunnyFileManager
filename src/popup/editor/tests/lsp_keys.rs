//! Purpose: the editor's language-server keys — a shortcut records the request for the worker, and an open LSP popup takes keys before the document.

use super::*;

#[test]
fn lsp_shortcuts_record_the_request_for_the_worker_without_editing() {
    use crate::models::{lsp::Intent, CompletionTrigger, LspFeature};
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("main.rs"));
    for (code, modifiers, expected) in [
        (
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Completion(CompletionTrigger::Manual)),
        ),
        (
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Hover),
        ),
        (
            KeyCode::Char(']'),
            KeyModifiers::CONTROL,
            Intent::Query(LspFeature::Definition),
        ),
        (
            KeyCode::Char('r'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
            Intent::Restart,
        ),
    ] {
        key(&mut app, code, modifiers);
        let editor = app.get_editor().expect("editor");
        assert!(editor.lsp.intent == expected, "{code:?} {modifiers:?}");
        assert_eq!(editor.lines, ["text"]);
    }
}

#[test]
fn an_open_lsp_popup_takes_keys_before_the_document() {
    use crate::models::lsp::Dialog;
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("text", temp.path().join("main.rs"));
    app.get_editor_mut().expect("editor").lsp.popup = Some(Dialog::Hover {
        text: "docs".into(),
        scroll: 0,
    });

    key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
    let editor = app.get_editor().expect("editor");
    assert_eq!(editor.lines, ["text"]);
    assert!(editor.lsp.popup.is_some());

    key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    let editor = app
        .get_editor()
        .expect("Esc closes the popup, not the editor");
    assert!(editor.lsp.popup.is_none());
    assert_eq!(editor.state, EditState::Clean);
}
