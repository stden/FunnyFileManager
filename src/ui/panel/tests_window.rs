//! Unit tests for the visible panel window.

use super::visible_window;

/// Nothing to show is not the same as showing nothing badly: an empty
/// listing and a zero-height body both give an empty range rather than a
/// slice that would panic.
#[test]
fn an_empty_listing_or_body_shows_nothing() {
    assert_eq!(visible_window(0, 40, 0, None), 0..0);
    assert_eq!(visible_window(100, 0, 0, Some(5)), 0..0);
    assert_eq!(visible_window(0, 0, 0, None), 0..0);
}

/// A listing shorter than the body is shown whole, from the top.
#[test]
fn a_short_listing_fills_from_the_top() {
    assert_eq!(visible_window(5, 40, 0, Some(2)), 0..5);
    // …even if a stale offset survived a directory that just shrank.
    assert_eq!(visible_window(5, 40, 900, Some(2)), 0..5);
}

#[test]
fn a_long_listing_shows_exactly_one_body() {
    assert_eq!(visible_window(1_000, 40, 0, Some(0)), 0..40);
    assert_eq!(visible_window(1_000, 40, 100, Some(120)), 100..140);
}

/// The offset holds while the cursor is inside it. Scrolling on every
/// keypress instead would make the listing jump under the user.
#[test]
fn the_offset_holds_while_the_cursor_is_visible() {
    for selected in 100..140 {
        assert_eq!(
            visible_window(1_000, 40, 100, Some(selected)),
            100..140,
            "cursor at {selected}"
        );
    }
}

/// Past the bottom, the window moves by exactly one row — Far scrolls a
/// line at a time, not half a page.
#[test]
fn stepping_past_the_bottom_scrolls_one_row() {
    assert_eq!(visible_window(1_000, 40, 100, Some(140)), 101..141);
    assert_eq!(visible_window(1_000, 40, 100, Some(141)), 102..142);
}

/// And past the top the same, upwards.
#[test]
fn stepping_past_the_top_scrolls_one_row() {
    assert_eq!(visible_window(1_000, 40, 100, Some(99)), 99..139);
    assert_eq!(visible_window(1_000, 40, 100, Some(0)), 0..40);
}

/// A jump to the end shows the last body-full, with no blank rows under it.
#[test]
fn the_end_of_the_listing_has_no_blank_rows_below_it() {
    let window = visible_window(1_000, 40, 0, Some(999));
    assert_eq!(window, 960..1_000);
    assert_eq!(window.len(), 40);
}

/// The cursor is always inside the window it returns — that is the whole
/// job, and the one thing a scrolling bug breaks.
#[test]
fn the_cursor_is_always_inside_the_window() {
    for len in [1_usize, 5, 40, 41, 1_000] {
        for rows in [1_usize, 7, 40] {
            for offset in [0_usize, 1, 39, len.saturating_sub(1), len * 2] {
                assert_cursor_window(len, rows, offset);
            }
        }
    }
}

fn assert_cursor_window(len: usize, rows: usize, offset: usize) {
    for selected in [0, len / 2, len.saturating_sub(1)] {
        let window = visible_window(len, rows, offset, Some(selected));
        assert!(
            window.contains(&selected),
            "len {len}, rows {rows}, offset {offset}, selected {selected}: got {window:?}"
        );
        assert!(window.end <= len, "past the end: {window:?}");
        assert!(window.len() <= rows, "taller than the body: {window:?}");
    }
}

/// A selection past the end of a listing that shrank is ignored rather
/// than obeyed: obeying it would scroll to a row that is not there.
#[test]
fn a_stale_selection_does_not_scroll_past_the_end() {
    let window = visible_window(10, 40, 0, Some(900));
    assert_eq!(window, 0..10);
}
