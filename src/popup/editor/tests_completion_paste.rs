//! A paste dismisses completion and inserts text; stale input cannot open an editor.

use super::*;
use crate::lsp::types::{CompletionTrigger, Feature, Query};
use crate::models::lsp::Dialog;
use crate::models::{Overlay, Panel};
use crate::popup::tests::app_with_editor;

#[test]
fn pasting_dismisses_completion_and_inserts_the_text_at_the_caret() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = app_with_editor("hel", dir.path().join("main.rs"));
    let editor = app.get_editor_mut().expect("editor");
    editor.cursor_col = 3;
    let query = Query {
        stamp: editor.lsp_stamp(),
        position: editor.lsp_position().expect("position"),
        feature: Feature::Completion(CompletionTrigger::Manual),
    };
    editor.lsp.popup = Some(Dialog::Completion {
        query,
        items: Vec::new(),
        selected: 0,
    });
    paste_text(&mut app, "p");
    let editor = app.get_editor().expect("editor");
    assert!(editor.lsp.popup.is_none());
    assert_eq!(editor.content(), "help");
}

#[test]
fn paste_and_wheel_do_not_change_the_app_when_no_editor_is_open() {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    paste_text(&mut app, "ignored");
    assert!(!wheel(
        &mut app,
        crate::mouse::Wheel::Up,
        KeyModifiers::NONE
    ));
    assert!(matches!(app.overlay, Overlay::None));
}
