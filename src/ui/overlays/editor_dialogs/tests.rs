//! Purpose: Verify the editor search dialog leaves the cursor unset when a non-text control is focused.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, layout::Position, Terminal};

use super::render_editor_dialog;
use crate::models::Editor;
use crate::mouse::DialogMouse;
use crate::search::{SearchField, SearchPrompt};

#[test]
fn non_query_search_focus_does_not_place_the_text_caret() {
    let mut editor = Editor::from_content(
        "main.rs".to_owned(),
        PathBuf::from("main.rs"),
        "fn main() {}\n",
    );
    editor.search.prompt = Some(SearchPrompt {
        field: SearchField::Case,
        ..SearchPrompt::default()
    });
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    let mut drawn = false;
    terminal
        .draw(|frame| drawn = render_editor_dialog(frame, &mut tracker, &editor, (0, 0)))
        .expect("draw");
    assert!(drawn);
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(0, 0)
    );
}
