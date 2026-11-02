//! Purpose: An empty selection names no owner and no group.

use crate::models::AttributesDialog;

#[test]
fn an_empty_selection_names_no_owner_or_group() {
    let dialog = AttributesDialog::from_snapshots(&[]);
    assert_eq!(dialog.owner, "");
    assert_eq!(dialog.group, "");
}
