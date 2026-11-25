//! Purpose: Unit tests for `retarget` preserving unindexed rows and updating indexed ones.

use std::collections::HashMap;

use super::super::super::tests_support::{process, snapshot};
use super::super::super::tree::TreeLine;
use super::{retarget, ViewRow};

#[test]
fn retarget_updates_present_keys_and_preserves_missing_rows() {
    let snap = snapshot(vec![process(1, "one"), process(2, "two")]);
    let rows = snap.rows();
    let mut view = vec![
        ViewRow::new(0, &rows[0], TreeLine::Flat),
        ViewRow::new(1, &rows[1], TreeLine::Flat),
    ];
    let indexes = HashMap::from([(rows[0].sample().key, 5)]);

    retarget(&mut view, &indexes);

    assert_eq!(view[0].row(), 5);
    assert_eq!(view[1].row(), 1);
}
