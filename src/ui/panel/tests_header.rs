//! Sort-mode header coverage preserved from upstream.

use super::layout::PanelLayout;
use super::*;
use crate::models::{PanelView, SortMode};

fn layout() -> PanelLayout {
    panel_layout(PanelView::Brief, Cells(62))
}

/// The header's leading letter names the sort mode — the only place the
/// panel says how it is ordered. `Name` is exercised by every other
/// rendering test in this file; `Extension` and `Size` are not, since a
/// panel under test is almost always sorted by name.
#[test]
fn the_leading_letter_names_every_sort_mode() {
    let letters: String = SortMode::ALL.iter().map(|mode| mode.letter()).collect();
    assert_eq!(letters, "nemsucado", "Far's letters in Far's menu order");
    for (mode, letter) in SortMode::ALL.iter().map(|mode| (*mode, mode.letter())) {
        let line = header_line(mode, layout());
        let first_span = line.spans.first().expect("the name column has a span");
        assert!(
            first_span.content.starts_with(letter),
            "sort mode {mode:?} should start the header with {letter:?}, got {:?}",
            first_span.content
        );
    }
}
