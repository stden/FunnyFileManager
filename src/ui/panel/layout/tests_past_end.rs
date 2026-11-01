//! Purpose: A column past the last stripe belongs to the last stripe.

use super::*;

#[test]
fn a_column_past_the_last_stripe_belongs_to_the_last_stripe() {
    let interior = Cells::new(60);
    let stripes = panel_layout(PanelView::Brief, interior).stripes;
    assert_eq!(
        stripe_at(PanelView::Brief, interior, usize::MAX / 2),
        stripes - 1
    );
}
