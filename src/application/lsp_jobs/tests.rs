//! Purpose: Ensure background results cannot update an unrelated editor or older revision.

use super::*;
use crate::{
    lsp::{
        features::{Diagnostic, Reply},
        positions::Range,
        types::Feature,
    },
    models::{Editor, Overlay, Panel},
};
use tempfile::TempDir;

fn app(root: &TempDir) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "main.rs".into(),
        root.path().join("main.rs"),
        "fn main() {}",
    )));
    app
}

#[test]
fn obsolete_document_and_version_diagnostics_are_ignored() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let stamp = app.get_editor().unwrap().lsp_stamp();
    let diagnostics = vec![Diagnostic {
        range: Range::default(),
        message: "error".into(),
        severity: Some(1),
    }];
    assert!(apply_event(
        &mut app,
        Envelope(stamp.id, Event::Diagnostics(stamp, diagnostics.clone()))
    ));
    assert_eq!(app.get_editor().unwrap().lsp.diagnostics.len(), 1);
    app.get_editor_mut().unwrap().insert_char('x');
    assert!(!apply_event(
        &mut app,
        Envelope(stamp.id, Event::Diagnostics(stamp, diagnostics))
    ));
    assert!(app.get_editor().unwrap().lsp.diagnostics.is_empty());
    assert!(!apply_event(
        &mut app,
        Envelope(
            crate::lsp::types::DocumentId::default(),
            Event::State(ServerState::Ready)
        )
    ));
}

#[test]
fn disabled_configuration_never_starts_a_worker() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    app.config.lsp.0.clear();
    let mut controller = EditorLspController::default();
    assert!(!controller.pump(&mut app));
    assert!(controller.manager.is_none());
}

#[test]
fn cursor_movement_prevents_hover_from_opening() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: Position::default(),
        feature: Feature::Hover,
    };
    editor.move_right();
    apply_event(
        &mut app,
        Envelope(
            query.stamp.id,
            Event::Reply(query, Ok(Reply::Hover("old".into()))),
        ),
    );
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
}
#[test]
fn disabled_and_failed_servers_explain_why_queries_cannot_run() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    app.config.lsp.0.clear();
    app.get_editor_mut().unwrap().lsp.intent = Intent::Query(Feature::Hover);
    let mut controller = EditorLspController::default();
    assert!(controller.pump(&mut app));
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("LSP is disabled for this file")
    );
    let editor = app.get_editor_mut().unwrap();
    editor.lsp.state = ServerState::Failed;
    editor.lsp.intent = Intent::Query(Feature::Hover);
    assert!(requested_query(editor, Context::from_editor(editor)).is_none());
    assert!(editor.notice.as_ref().unwrap().contains("restart"));
}
