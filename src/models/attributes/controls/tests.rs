//! Purpose: Unit tests for out-of-range flag toggle and non-time-field backspace guards.

use super::{AttributesDialog, AttributesField};

#[test]
fn toggle_ignores_out_of_bounds_flag_and_backspace_ignores_non_time_fields() {
    let mut dialog = AttributesDialog::default();
    dialog.modified.current = "2026-01-01 00:00:00".to_owned();
    dialog.field = AttributesField::Flag(usize::MAX);
    dialog.toggle();

    dialog.field = AttributesField::Set;
    dialog.backspace();
    assert_eq!(dialog.modified.current, "2026-01-01 00:00:00");
}
