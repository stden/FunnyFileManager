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
