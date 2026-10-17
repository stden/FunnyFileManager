//! Purpose: Verify automatic workspace approval starts servers without a modal and rejects stale events.

use super::super::*;
use crate::{
    lsp::manager::{Envelope, Event},
    models::{lsp::Dialog, Editor, Overlay},
};
use tempfile::TempDir;

fn automatic_app(root: &TempDir) -> App {
    let mut app = tests_session::fixture(root);
    let settings: crate::config::Settings =
        serde_norway::from_str("editor: {lsp_trust: automatic}").unwrap();
    app.config.editor = settings.editor;
    app
}

#[test]
fn automatic_trust_approves_only_the_current_editor_without_a_popup() {
    let root = TempDir::new().unwrap();
    let mut app = automatic_app(&root);
    let owner = app.get_editor().unwrap().lsp.id;
    assert!(apply_event(
        &mut app,
        Envelope(owner, Event::TrustRequired(root.path().to_path_buf()))
    ));
    let editor = app.get_editor().unwrap();
    assert!(editor.lsp.popup.is_none());
    assert_eq!(editor.lsp.state, ServerState::Starting);
    assert!(editor.lsp.intent == Intent::Trust);
    app.get_editor_mut().unwrap().lsp.intent = Intent::None;
    assert!(!apply_event(
        &mut app,
        Envelope(
            crate::lsp::types::DocumentId::default(),
            Event::TrustRequired(root.path().to_path_buf())
        )
    ));
    assert!(app.get_editor().unwrap().lsp.intent == Intent::None);
}

fn ready_without_a_trust_prompt(controller: &mut EditorLspController, app: &mut App) {
    tests_session::pump_until(controller, app, |editor| {
        editor.lsp.state == ServerState::Ready
            || matches!(editor.lsp.popup, Some(Dialog::Trust { .. }))
    });
    assert_eq!(app.get_editor().unwrap().lsp.state, ServerState::Ready);
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
}

#[test]
fn automatic_trust_starts_restarts_and_opens_another_workspace_without_confirmation() {
    let root = TempDir::new().unwrap();
    let mut app = automatic_app(&root);
    let mut controller = EditorLspController::default();
    ready_without_a_trust_prompt(&mut controller, &mut app);
    app.get_editor_mut().unwrap().lsp.intent = Intent::Restart;
    controller.pump(&mut app);
    ready_without_a_trust_prompt(&mut controller, &mut app);
    let other = TempDir::new().unwrap();
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "other.rs".into(),
        other.path().join("other.rs"),
        "fn other() {}",
    )));
    ready_without_a_trust_prompt(&mut controller, &mut app);
    app.close_overlay();
    controller.pump(&mut app);
}

#[test]
fn settings_without_automatic_trust_keep_the_confirmation_prompt() {
    let root = TempDir::new().unwrap();
    let mut app = tests_session::fixture(&root);
    let owner = app.get_editor().unwrap().lsp.id;
    apply_event(
        &mut app,
        Envelope(owner, Event::TrustRequired(root.path().to_path_buf())),
    );
    let editor = app.get_editor().unwrap();
    assert!(matches!(
        editor.lsp.popup,
        Some(Dialog::Trust { allow: false, .. })
    ));
    assert_eq!(editor.lsp.state, ServerState::Untrusted);
    assert!(editor.lsp.intent == Intent::None);
}
