//! Purpose: A bracket pair scrolled off to the left, or on a line the editor no
//! longer has, paints nothing; the same pair paints once the scroll allows it.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::{paint_span, TagSpan};
use crate::models::Editor;
use crate::ui::palette;
use crate::ui::Cells;

/// How many cells `span` paints in the URL colour, with `skip` cells scrolled
/// off the left edge, in an editor holding `<b>text</b>` on its one line.
fn painted_cells(span: TagSpan, skip: usize) -> usize {
    let editor = Editor::from_content(
        "page.html".into(),
        PathBuf::from("page.html"),
        "<b>text</b>",
    );
    let mut terminal = Terminal::new(TestBackend::new(12, 6)).expect("terminal");
    terminal
        .draw(|f| paint_span(f, &editor, f.area(), 0, Cells::new(skip), span))
        .expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .filter(|cell| cell.fg == palette::CODE_URL)
        .count()
}

#[test]
fn a_tag_scrolled_off_to_the_left_paints_nothing() {
    assert_eq!(painted_cells(TagSpan::new(0, 0, 3), 5), 0);
    assert_eq!(
        painted_cells(TagSpan::new(0, 0, 3), 0),
        3,
        "the same pair is painted unscrolled"
    );
}

#[test]
fn a_span_on_a_line_the_editor_no_longer_has_paints_nothing() {
    assert_eq!(painted_cells(TagSpan::new(5, 0, 3), 0), 0);
}
