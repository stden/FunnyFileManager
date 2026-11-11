//! Purpose: A path entry too narrow for its field draws no text in it, and a
//! wide enough one shows the entry being edited.

use ratatui::layout::Rect;
use ratatui::Terminal;

use super::{footer, render_editor};
use crate::path_env::{Mode, PathEditor, Scope, Snapshot};
use crate::test_support::{buffer_text, WritableTestBackend};

fn editing() -> PathEditor {
    let snapshot = Snapshot {
        value: "/usr/bin".to_owned(),
        revision: String::new(),
        location: String::new(),
    };
    let mut editor = PathEditor::new(Scope::User, snapshot);
    editor.edit(Some(0));
    editor
}

#[test]
fn an_entry_too_narrow_for_its_field_shows_no_text() {
    let editor = editing();
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 6)).expect("terminal");
    terminal
        .draw(|f| render_editor(f, &editor, Rect::new(0, 0, 13, 4), 0))
        .expect("draw");
    assert!(!buffer_text(&terminal).contains("/usr/bin"));
}

#[test]
fn a_wide_entry_shows_the_path_being_edited() {
    let editor = editing();
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 6)).expect("terminal");
    terminal
        .draw(|f| render_editor(f, &editor, Rect::new(0, 0, 40, 4), 0))
        .expect("draw");
    assert!(buffer_text(&terminal).contains("/usr/bin"));
}

fn footer_text(editor: &PathEditor) -> String {
    footer(editor)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_footer_asks_before_saving_or_discarding() {
    let mut editor = editing();
    editor.mode = Mode::ConfirmSave;
    assert!(footer_text(&editor).contains("Save PATH"));
    editor.mode = Mode::ConfirmDiscard;
    assert!(footer_text(&editor).contains("Discard unsaved PATH"));
}
