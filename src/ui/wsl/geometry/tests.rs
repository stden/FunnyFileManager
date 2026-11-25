//! Purpose: Verify empty control slices produce no wrapped button rows.

use super::control_rows;

#[test]
fn empty_controls_produce_no_rows() {
    assert!(control_rows(&[], 10).is_empty());
}
