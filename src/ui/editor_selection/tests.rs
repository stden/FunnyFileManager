//! Selection rendering respects Unicode cell widths and horizontal clipping.

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use tempfile::TempDir;

use crate::models::{App, EditorMotion, Overlay, SelectionMode};

use super::*;

#[test]
fn a_selected_newline_left_of_the_viewport_is_not_painted_at_column_zero() {
    let temp = TempDir::new().expect("temporary directory");
    let mut editor = Editor::from_content(
        "text".to_owned(),
        temp.path().join("text.txt"),
        "a\n0123456789",
    );
    editor.move_cursor(EditorMotion::Down(1), SelectionMode::Extend);
    editor.move_cursor(EditorMotion::End, SelectionMode::Extend);
    let buffer = draw(editor, 6);
    for x in 0..6 {
        assert_eq!(buffer[(x, 1)].bg, crate::ui::FAR_BLUE);
    }
}

fn draw(editor: Editor, width: u16) -> Buffer {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Editor(editor));
    let mut terminal = Terminal::new(TestBackend::new(width, 8)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("render");
    terminal.backend().buffer().clone()
}

#[test]
fn reverse_multiline_selection_paints_wide_characters_and_line_breaks() {
    let temp = TempDir::new().expect("temporary directory");
    let mut editor = Editor::from_content(
        "text".to_owned(),
        temp.path().join("text.txt"),
        "a界\nβ🙂\nz!",
    );
    editor.cursor_line = 2;
    editor.cursor_col = 1;
    editor.move_cursor(EditorMotion::Up(2), SelectionMode::Extend);
    let buffer = draw(editor, 40);
    assert_eq!(buffer[(0, 1)].bg, crate::ui::FAR_BLUE);
    // BufferDiff emits a wide glyph only at its leading cell. Its style
    // covers both terminal columns; TestBackend leaves the continuation at
    // Reset because it receives no separate write for that covered column.
    assert_eq!(buffer[(1, 1)].symbol(), "界");
    assert_eq!(buffer[(1, 2)].symbol(), "🙂");
    for cell in [(1, 1), (3, 1), (0, 2), (1, 2), (3, 2), (0, 3)] {
        assert_eq!(
            buffer[cell].bg,
            palette::CURSOR_BG,
            "selected cell {cell:?}"
        );
    }
    assert_eq!(buffer[(1, 3)].bg, crate::ui::FAR_BLUE);
}

#[test]
fn selection_at_a_horizontally_scrolled_wide_character_uses_cell_offsets() {
    let temp = TempDir::new().expect("temporary directory");
    let mut editor = Editor::from_content(
        "text".to_owned(),
        temp.path().join("text.txt"),
        "aaaa界🙂zzz",
    );
    editor.cursor_col = 4;
    editor.move_cursor(EditorMotion::Right, SelectionMode::Extend);
    editor.move_cursor(EditorMotion::Right, SelectionMode::Extend);
    let buffer = draw(editor, 6);
    assert_eq!(buffer[(0, 1)].symbol(), "a");
    assert_eq!(buffer[(0, 1)].bg, crate::ui::FAR_BLUE);
    assert_eq!(buffer[(1, 1)].symbol(), "界");
    assert_eq!(buffer[(3, 1)].symbol(), "🙂");
    for x in [1, 3] {
        assert_eq!(buffer[(x, 1)].bg, palette::CURSOR_BG);
    }
    assert_eq!(buffer[(5, 1)].bg, crate::ui::FAR_BLUE);
}
