//! Purpose: Verify command selections use display cells and stay inside the visible input area.

use super::*;
use crate::lineedit::LineEditor;
use ratatui::{backend::TestBackend, Terminal};

fn selected_columns(line: &LineEditor, area: Rect, typed_from: usize, caret: usize) -> Vec<u16> {
    let mut terminal = Terminal::new(TestBackend::new(14, 3)).unwrap();
    terminal
        .draw(|frame| {
            let screen = frame.area();
            frame
                .buffer_mut()
                .set_style(screen, Style::default().bg(Color::Blue));
            paint_selection(frame, area, line, Cells::new(typed_from), Cells::new(caret));
        })
        .unwrap();
    (0..14)
        .filter(|column| {
            let cell = &terminal.backend().buffer()[(*column, area.y)];
            if cell.bg != Color::Cyan {
                return false;
            }
            assert_eq!(cell.fg, Color::Black);
            true
        })
        .collect()
}

#[test]
fn a_backward_selection_paints_unicode_display_cells_after_the_prompt() {
    let mut line = LineEditor::new("é😀z");
    line.select_all();
    assert_eq!(
        selected_columns(&line, Rect::new(3, 1, 8, 1), 2, 6),
        vec![5, 6, 7, 8]
    );
}

#[test]
fn a_forward_selection_is_clipped_at_the_right_edge_of_the_row() {
    let mut line = LineEditor::new("abcdef");
    line.extend(LineEditor::home);
    assert_eq!(
        selected_columns(&line, Rect::new(3, 1, 5, 1), 2, 2),
        vec![5, 6, 7]
    );
}

#[test]
fn a_selection_scrolled_off_the_left_edge_never_paints_the_prompt() {
    let mut line = LineEditor::new("abcdefghij");
    line.select_all();
    assert_eq!(
        selected_columns(&line, Rect::new(3, 1, 6, 1), 2, 5),
        vec![5, 6, 7]
    );
}

#[test]
fn an_unselected_line_keeps_the_background_style() {
    let line = LineEditor::new("plain");
    assert_eq!(
        selected_columns(&line, Rect::new(3, 1, 8, 1), 2, 7).len(),
        0
    );
}
