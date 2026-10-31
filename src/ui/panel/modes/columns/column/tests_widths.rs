//! Purpose: The byte figures a column holds: a short figure is written exactly,
//! a long one is abbreviated, and the modes that show no columns show none.

use super::*;

#[test]
fn a_short_size_is_written_exactly_and_a_long_one_is_abbreviated() {
    assert_eq!(fitted_bytes(512), "512");
    assert_eq!(fitted_bytes(1_234_567), abbreviate_bytes(1_234_567));
}

#[test]
fn a_grouped_size_too_wide_for_its_column_is_abbreviated() {
    assert_eq!(grouped_bytes(7), "7");
    assert_eq!(
        grouped_bytes(10_000_000_000),
        abbreviate_bytes(10_000_000_000)
    );
}

#[test]
fn the_modes_that_draw_no_columns_have_none() {
    for view in [
        PanelView::Brief,
        PanelView::Medium,
        PanelView::Full,
        PanelView::Wide,
    ] {
        assert!(columns(view).is_empty());
    }
}
