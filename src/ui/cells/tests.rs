//! Purpose: Pin cell-coordinate conversion, combining-mark clipping, and editor line measurement.

use crate::models::{CharColumn, TabSize};

use super::{
    expand_tabs, middle_cells, screen_u16, skip_cells, tail_cells, Cells, EditorLine, UNPRINTABLE,
};

#[test]
fn screen_u16_keeps_every_value_a_terminal_can_address() {
    assert_eq!(screen_u16(0), 0);
    assert_eq!(screen_u16(80), 80);
    assert_eq!(screen_u16(usize::from(u16::MAX)), u16::MAX);
}

/// `usize` has no negative values; overflow is the only way out of range.
#[test]
fn screen_u16_saturates_past_the_last_coordinate_instead_of_wrapping() {
    assert_eq!(screen_u16(usize::from(u16::MAX) + 1), u16::MAX);
    assert_eq!(screen_u16(usize::MAX), u16::MAX);
}

#[test]
fn editor_line_counts_unsafe_control_and_bidi_chars_as_one_cell_replacement() {
    let text = "\x07\u{202e}\tx";
    let line = EditorLine {
        text,
        tab: TabSize::DEFAULT,
    };
    let drawn = expand_tabs(text, Cells::ZERO, TabSize::DEFAULT);
    assert_eq!(drawn, format!("{UNPRINTABLE}{UNPRINTABLE}  x"));
    assert_eq!(line.cells_before(CharColumn::new(1)), Cells::new(1));
    assert_eq!(line.cells_before(CharColumn::new(2)), Cells::new(2));
    assert_eq!(line.cells_before(CharColumn::new(3)), Cells::new(4));
    assert_eq!(line.width(), Cells::new(5));
    assert_eq!(line.dropped(Cells::new(1)), Cells::new(1));
}

#[test]
fn skip_cells_drops_combining_marks_attached_to_skipped_base_char() {
    assert_eq!(
        skip_cells("e\u{0301}\u{0302}x", Cells::new(1)),
        ("x", Cells::new(1))
    );
    assert_eq!(
        skip_cells("界\u{0301}x", Cells::new(1)),
        ("x", Cells::new(2))
    );
    assert_eq!(
        skip_cells("\u{0301}x", Cells::ZERO),
        ("\u{0301}x", Cells::ZERO)
    );
}

#[test]
fn tail_cells_drops_orphaned_combining_marks_when_base_char_is_cut() {
    assert_eq!(tail_cells("abcde\u{0301}f", Cells::new(2)), "…f");
    assert_eq!(tail_cells("abcde\u{0301}f", Cells::new(3)), "…e\u{0301}f");
}

#[test]
fn middle_cells_keeps_the_start_and_the_end_around_an_ellipsis() {
    assert_eq!(middle_cells("/a/b/c/file.txt", Cells::new(9)), "/a/b….txt");
    assert_eq!(middle_cells("short", Cells::new(9)), "short");
    assert_eq!(
        middle_cells("/dir/日本語", Cells::new(5)),
        "/d…語",
        "wide characters are measured in cells"
    );
    assert_eq!(middle_cells("abcdef", Cells::ZERO), "");
}

#[test]
fn highlighted_window_expands_tabs_to_tab_stops() {
    let cache = std::cell::RefCell::new(crate::highlight::cache::HighlightCache::default());
    let lines = vec!["a\tb".to_owned(), "ab\tc".to_owned()];
    let rendered = super::super::code::highlighted_window(
        &cache,
        std::path::Path::new("sample.txt"),
        &lines,
        0,
        2,
        Cells::ZERO,
        Cells::new(20),
    );
    let first: String = rendered[0]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    let second: String = rendered[1]
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect();
    assert_eq!(first, "a   b");
    assert_eq!(second, "ab  c");
}
