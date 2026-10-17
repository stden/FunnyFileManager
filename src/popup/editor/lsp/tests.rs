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
fn popup_keys_filter_while_typing_widen_on_backspace_and_apply_with_enter_or_tab() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    completion(&mut app);
    for key in [KeyCode::Down, KeyCode::Down, KeyCode::Up] {
        assert!(process(&mut app, key));
    }
    assert!(process(&mut app, KeyCode::Char('p')));
    assert_eq!(app.get_editor().unwrap().content(), "help");
    assert!(process(&mut app, KeyCode::Backspace));
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
fn tab_replaces_identifier_suffix_while_enter_preserves_it() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let editor = app.get_editor_mut().unwrap();
    editor.lines = vec!["hel_old()".into()];
    editor.cursor_col = 3;
    completion(&mut app);
    assert!(process(&mut app, KeyCode::Tab));
    assert_eq!(app.get_editor().unwrap().content(), "hello()");

    let editor = app.get_editor_mut().unwrap();
    editor.lines = vec!["hel_old()".into()];
    editor.cursor_col = 3;
    completion(&mut app);
    assert!(process(&mut app, KeyCode::Enter));
    assert_eq!(app.get_editor().unwrap().content(), "hello_old()");
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
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
    let target = features::Definition {
        path: app.get_editor().unwrap().path.clone(),
        range: crate::lsp::positions::Range::default(),
    };
    let mut second = target.clone();
    second.range.start.character = 1;
    second.range.end.character = 1;
    app.get_editor_mut().unwrap().lsp.popup = Some(Dialog::Definitions {
        items: vec![target, second],
        selected: 0,
    });
    process(&mut app, KeyCode::Down);
    process(&mut app, KeyCode::Enter);
    assert_eq!(app.get_editor().unwrap().cursor_col, 1);
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

#[test]
fn workspace_trust_defaults_to_denial_and_requires_selecting_trust() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    for accept in [false, true] {
        app.get_editor_mut()
            .unwrap()
            .lsp
            .ask_trust(root.path().to_path_buf());
        if accept {
            process(&mut app, KeyCode::Down);
        }
        process(&mut app, KeyCode::Enter);
        let intent = app.get_editor().unwrap().lsp.intent;
        assert_eq!(intent == crate::models::lsp::Intent::Trust, accept);
        assert!(app.get_editor().unwrap().lsp.popup.is_none());
    }
}
