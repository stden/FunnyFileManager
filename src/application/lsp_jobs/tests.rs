//! Purpose: Ensure background results cannot update an unrelated editor or older revision.

use super::*;
use crate::lsp::manager::{Envelope, Event};
use crate::{
    lsp::{
        features::{Diagnostic, Reply},
        positions::Range,
        types::Feature,
    },
    models::{Editor, Overlay, Panel},
};
use tempfile::TempDir;

mod trust;

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
    assert_eq!(app.get_editor().unwrap().lsp.diagnostics.len(), 0);
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
    editor.notice = Some("Newer request".into());
    apply_event(
        &mut app,
        Envelope(
            query.stamp.id,
            Event::Reply(query, Ok(Reply::Hover("old".into()))),
        ),
    );
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Newer request")
    );
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

#[test]
fn request_errors_are_visible_only_for_the_current_cursor_and_revision() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: Position::default(),
        feature: Feature::Hover,
    };
    let reply = Event::Reply(
        query,
        Err(serde_json::from_value(serde_json::json!({
            "code": -32001, "message": "failure"
        }))
        .unwrap()),
    );
    assert!(apply_event(
        &mut app,
        Envelope(query.stamp.id, reply.clone())
    ));
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Language-server request failed (-32001)")
    );
    app.get_editor_mut().unwrap().move_right();
    app.get_editor_mut().unwrap().notice = Some("newer feedback".into());
    assert!(!apply_event(&mut app, Envelope(query.stamp.id, reply)));
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("newer feedback")
    );
}

#[test]
fn pending_commands_coalesce_and_clear_on_close() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let mut controller = EditorLspController::default();
    let now = Instant::now();
    let editor = app.get_editor_mut().unwrap();
    let initial = Context::from_editor(editor);
    controller.observed = Some(initial);
    controller.sent = Some((initial.stamp, initial.saved));

    editor.insert_char('a');
    let c1 = Context::from_editor(editor);
    controller.update(editor, c1, now);
    controller.update(editor, c1, now + Duration::from_millis(200));
    editor.insert_char('b');
    let c2 = Context::from_editor(editor);
    controller.update(editor, c2, now + Duration::from_millis(250));
    controller.update(editor, c2, now + Duration::from_millis(400));

    assert_eq!(
        controller.pending.len(),
        2,
        "one Cancel and one latest Update"
    );
    assert!(matches!(controller.pending[0], Command::Cancel));
    assert!(matches!(controller.pending[1], Command::Update(_)));

    controller.close();
    assert_eq!(controller.pending.len(), 1);
    assert!(matches!(controller.pending[0], Command::Close));
}

#[test]
fn auto_query_clears_when_not_ready_and_emits_when_ready() {
    let root = TempDir::new().unwrap();
    let mut app = app(&root);
    let editor = app.get_editor_mut().unwrap();
    let ctx = Context::from_editor(editor);

    editor.lsp.auto_complete = true;
    editor.lsp.state = ServerState::Starting;
    assert!(events::auto_query(editor, ctx, true).is_none());
    assert!(!editor.lsp.auto_complete);
    editor.lsp.auto_complete = true;
    editor.lsp.state = ServerState::Ready;
    assert!(matches!(
        events::auto_query(editor, ctx, true),
        Some(Command::Query(Query {
            feature: Feature::Completion(crate::lsp::types::CompletionTrigger::AutoWhileTyping),
            ..
        }))
    ));
}
