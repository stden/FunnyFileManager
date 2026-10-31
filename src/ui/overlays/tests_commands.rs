//! Purpose: The Commands-menu renderer draws nothing for an overlay it does not
//! own, and leaves the mouse map untouched.

use ratatui::Terminal;

use super::{render_commands_dialog, render_overlays};
use crate::models::{App, Overlay, Panel};
use crate::mouse::DialogMouse;
use crate::path_env::{PathEditor, Scope, Snapshot};
use crate::test_support::{buffer_text, WritableTestBackend};

#[test]
fn an_overlay_that_is_not_a_commands_dialog_draws_nothing() {
    let mut terminal = Terminal::new(WritableTestBackend::new(20, 4)).expect("terminal");
    let mut mouse = DialogMouse::default();
    terminal
        .draw(|f| render_commands_dialog(f, &mut Overlay::None, &mut mouse))
        .expect("draw");
    assert_eq!(buffer_text(&terminal).trim(), "");
}

#[test]
fn the_path_editor_overlay_is_drawn_with_its_entries() {
    let root = tempfile::tempdir().expect("test directory");
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).expect("test panel"),
        Panel::new(root.path().to_path_buf()).expect("test panel"),
    );
    let snapshot = Snapshot {
        value: "/usr/bin".to_owned(),
        revision: String::new(),
        location: String::new(),
    };
    app.set_overlay(Overlay::PathEditor(Box::new(PathEditor::new(
        Scope::User,
        snapshot,
    ))));
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    assert!(buffer_text(&terminal).contains("/usr/bin"));
}
