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

#[test]
fn completion_navigation_handles_an_empty_result_without_selecting_a_missing_item() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    completion(&mut app);
    let mut dialog = app.get_editor_mut().unwrap().lsp.popup.take().unwrap();
    navigate(&mut dialog, true);
    assert!(matches!(dialog, Dialog::Completion { selected: 1, .. }));
    let Dialog::Completion {
        items, selected, ..
    } = &mut dialog
    else {
        panic!("completion");
    };
    items.clear();
    *selected = 0;
    navigate(&mut dialog, true);
    assert!(matches!(dialog, Dialog::Completion { selected: 0, .. }));
}

#[test]
fn empty_definition_selection_does_nothing() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    accept_modal(
        &mut app,
        Dialog::Definitions {
            items: Vec::new(),
            selected: 0,
        },
    );
    assert!(app.get_editor().unwrap().lsp.popup.is_none());
}

#[test]
fn closed_editor_ignores_completion_and_modal_helpers() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let editor = app.get_editor().unwrap();
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().unwrap(),
        feature: Feature::Completion(CompletionTrigger::Manual),
    };
    app.close_overlay();
    assert_eq!(
        process_completion(
            &mut app,
            query,
            Vec::new(),
            0,
            KeyCode::Down,
            KeyModifiers::NONE
        ),
        Some(true)
    );
    let hover = Dialog::Hover {
        text: "docs".to_owned(),
        scroll: 0,
    };
    process_modal_dialog(&mut app, hover, KeyCode::Down);
}

#[test]
fn closed_editor_ignores_trust_accept() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    app.close_overlay();
    accept_modal(
        &mut app,
        Dialog::Trust {
            root: root.path().to_path_buf(),
            allow: true,
        },
    );
    assert!(app.get_editor().is_none());
}
