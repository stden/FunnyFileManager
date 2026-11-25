//! Purpose: Which rows of a diagnostic fall inside the viewport: none when the
//! viewport is empty or the range lies wholly above or below it, and a range
//! that crosses an edge is clipped to the visible rows.

use super::*;
use crate::lsp::positions::{Position, Range};

fn spanning(start: u32, end: u32) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: start,
                character: 0,
            },
            end: Position {
                line: end,
                character: 0,
            },
        },
        message: String::new(),
        severity: None,
    }
}

#[test]
fn an_empty_viewport_shows_no_rows() {
    assert_eq!(visible_rows(&spanning(0, 3), 5, 5), 0..0);
}

#[test]
fn a_range_wholly_above_or_below_the_viewport_shows_no_rows() {
    assert_eq!(visible_rows(&spanning(0, 2), 5, 10), 0..0);
    assert_eq!(visible_rows(&spanning(12, 14), 5, 10), 0..0);
}

#[test]
fn a_range_crossing_the_viewport_edges_is_clipped_to_it() {
    assert_eq!(visible_rows(&spanning(3, 7), 5, 10), 5..8);
    assert_eq!(visible_rows(&spanning(7, 40), 5, 10), 7..10);
}

#[test]
fn diagnostics_skips_visible_rows_whose_columns_fall_outside_the_viewport() {
    use ratatui::{backend::TestBackend, style::Modifier, Terminal};
    use std::path::PathBuf;

    let mut editor = Editor::from_content("main.rs".to_owned(), PathBuf::from("main.rs"), "text");
    editor.lsp.diagnostics.push(Diagnostic {
        range: Range {
            start: Position::default(),
            end: Position {
                line: 0,
                character: 1,
            },
        },
        message: String::new(),
        severity: Some(1),
    });
    let mut terminal = Terminal::new(TestBackend::new(10, 2)).expect("terminal");
    terminal
        .draw(|frame| diagnostics(frame, &editor, frame.area(), 0, Cells::new(10)))
        .expect("draw");
    let underlined = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .any(|cell| cell.modifier.contains(Modifier::UNDERLINED));
    assert!(!underlined);
}
