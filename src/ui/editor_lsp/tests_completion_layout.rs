//! Purpose: Verify completion popups follow the caret while fitting above or below it on screen.

use super::*;
use crate::lsp::{
    features::Completion,
    positions::Position,
    types::{CompletionTrigger, Feature, Query},
};
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

#[test]
fn completion_popups_choose_the_side_with_room_and_stay_inside_the_viewport() {
    let screen = Rect::new(4, 3, 30, 12);
    for (caret, expected) in [
        ((8, 3), Rect::new(8, 4, 20, 6)),
        ((33, 14), Rect::new(14, 8, 20, 6)),
        ((8, 8), Rect::new(8, 9, 20, 6)),
        ((8, 9), Rect::new(8, 3, 20, 6)),
    ] {
        let area = completion_area(screen, caret, 20, 6);
        assert_eq!(area, expected);
        assert_eq!(area.intersection(screen), area);
    }
}

#[test]
fn completion_popups_shrink_to_the_available_space_on_tiny_screens() {
    let screen = Rect::new(2, 3, 5, 3);
    assert_eq!(
        completion_area(screen, (6, 4), 86, 18),
        Rect::new(2, 5, 5, 1)
    );
    assert_eq!(
        completion_area(screen, (6, 5), 86, 18),
        Rect::new(2, 3, 5, 2)
    );
    assert_eq!(
        completion_area(Rect::new(2, 3, 0, 0), (2, 3), 86, 18),
        Rect::new(2, 3, 0, 0)
    );
}

fn completion_editor(root: &TempDir) -> Editor {
    let mut editor = Editor::from_content("main.rs".into(), root.path().join("main.rs"), "");
    editor.lsp.popup = Some(Dialog::Completion {
        query: Query {
            stamp: editor.lsp_stamp(),
            position: Position::default(),
            feature: Feature::Completion(CompletionTrigger::Manual),
        },
        items: vec![Completion {
            label: "example".into(),
            insert_text: "example".into(),
            detail: "signature".into(),
            documentation: String::new(),
            kind: None,
            edit: None,
            additional_edits: Vec::new(),
            snippet: false,
        }],
        selected: 0,
    });
    editor
}

#[test]
fn a_completion_popup_renders_its_selected_item_above_a_bottom_row_caret() {
    let root = TempDir::new().unwrap();
    let editor = completion_editor(&root);
    let mut terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    terminal
        .draw(|frame| assert!(popup(frame, &editor, (28, 9))))
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(text.contains("> example []"));
    assert!(text.contains("signature"));
    assert_eq!(terminal.backend().buffer()[(0, 9)].symbol(), " ");
}

#[test]
fn an_editor_without_an_lsp_dialog_leaves_the_frame_untouched() {
    let root = TempDir::new().unwrap();
    let editor = Editor::from_content("main.rs".into(), root.path().join("main.rs"), "");
    let mut terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    terminal
        .draw(|frame| assert!(!popup(frame, &editor, (0, 0))))
        .unwrap();
    assert!(terminal
        .backend()
        .buffer()
        .content
        .iter()
        .all(|cell| cell.symbol() == " "));
}
