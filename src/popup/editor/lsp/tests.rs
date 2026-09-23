//! Purpose: Exercise modal LSP popup navigation, safe completion insertion, and dismissal.

use super::*;
use crate::{
    lsp::{
        features::{self, Reply},
        types::{Feature, Query},
    },
    models::{Editor, Overlay, Panel},
};
use tempfile::TempDir;

fn fixture(root: &TempDir) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "main.rs".into(),
        root.path().join("main.rs"),
        "hel",
    )));
    app.get_editor_mut().unwrap().cursor_col = 3;
    app
}

fn completion(app: &mut App) {
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: Feature::Completion,
    };
    let reply = features::parse(
        Feature::Completion,
        &serde_json::json!([{"label":"hello"},{"label":"help"}]),
    )
    .unwrap();
    editor.show_lsp_reply(query, reply);
}

#[test]
fn popup_keys_select_apply_and_dismiss_without_inserting_control_text() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    completion(&mut app);
    for key in [
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Char('x'),
    ] {
        assert!(process(&mut app, key));
    }
    assert_eq!(app.get_editor().unwrap().content(), "hel");
    assert!(process(&mut app, KeyCode::Enter));
    assert_eq!(app.get_editor().unwrap().content(), "hello");
    assert!(app.get_editor().unwrap().state.is_modified());
    completion(&mut app);
    assert!(process(&mut app, KeyCode::Esc));
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
    assert!(!process(&mut app, KeyCode::Esc));
}

#[test]
fn hover_scrolls_and_definition_picker_opens_the_selected_target() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    app.get_editor_mut().unwrap().lsp.popup = Some(Dialog::Hover {
        text: "details".into(),
        scroll: 0,
    });
    process(&mut app, KeyCode::Up);
    process(&mut app, KeyCode::Down);
    assert!(matches!(
        app.get_editor().unwrap().lsp.popup,
        Some(Dialog::Hover { scroll: 1, .. })
    ));
    process(&mut app, KeyCode::Enter);
    let target = features::Definition {
        path: app.get_editor().unwrap().path.clone(),
        range: crate::lsp::positions::Range::default(),
    };
    app.get_editor_mut().unwrap().lsp.popup = Some(Dialog::Definitions {
        items: vec![target.clone(), target],
        selected: 0,
    });
    process(&mut app, KeyCode::Down);
    process(&mut app, KeyCode::Enter);
    assert_eq!(app.get_editor().unwrap().cursor_col, 0);
    app.close_overlay();
    assert!(!process(&mut app, KeyCode::Enter));
}

#[test]
fn stale_completions_report_a_notice() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    completion(&mut app);
    app.get_editor_mut().unwrap().cursor_col = 0;
    process(&mut app, KeyCode::Enter);
    assert!(app
        .get_editor()
        .unwrap()
        .notice
        .as_ref()
        .unwrap()
        .contains("expired"));
}

#[test]
fn snippet_completions_report_a_notice() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: Feature::Completion,
    };
    let Reply::Completions(items) = features::parse(
        Feature::Completion,
        &serde_json::json!([{"label":"snippet","insertTextFormat":2}]),
    )
    .unwrap() else {
        panic!()
    };
    editor.lsp.popup = Some(Dialog::Completion {
        query,
        items,
        selected: 0,
    });
    process(&mut app, KeyCode::Enter);
    assert_eq!(
        app.get_editor().unwrap().notice.as_deref(),
        Some("Snippet completion is not supported")
    );
}
