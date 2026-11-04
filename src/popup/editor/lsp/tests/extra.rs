//! Purpose: Edge cases for empty completion accept, hover dismissal/keys, and modal helpers.

use super::*;

#[test]
fn empty_completion_enter_and_hover_escape_or_ignored_keys_stay_safe() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: Feature::Completion(CompletionTrigger::Manual),
    };
    editor.lsp.popup = Some(Dialog::Completion {
        query,
        items: Vec::new(),
        selected: 0,
    });
    assert!(process(&mut app, KeyCode::Enter));

    app.get_editor_mut().unwrap().lsp.popup = Some(Dialog::Hover {
        text: "docs".to_owned(),
        scroll: 0,
    });
    assert!(process(&mut app, KeyCode::Char('z')));
    assert!(process(&mut app, KeyCode::Esc));
}

#[test]
fn completion_helpers_ignore_empty_lists_and_closed_editors() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let editor = app.get_editor_mut().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: Feature::Completion(CompletionTrigger::Manual),
    };
    let mut comp = Dialog::Completion {
        query,
        items: Vec::new(),
        selected: 0,
    };
    navigate(&mut comp, true);
    accept_modal(&mut app, comp);

    let Reply::Completions(items) = features::parse(
        Feature::Completion(CompletionTrigger::Manual),
        &serde_json::json!([{"label": "item"}]),
    )
    .unwrap() else {
        panic!("completion");
    };
    app.close_overlay();
    dismiss_completion(&mut app, false);
    apply_completion(&mut app, query, &items, 0, false);
}
