//! Purpose: Verify `relabel` leaves the bar unchanged when a key is not in the row.

use super::{relabel, LIST_KEYS};

#[test]
fn relabel_leaves_the_bar_unchanged_when_the_key_is_absent() {
    let mut labels = LIST_KEYS;
    relabel(&mut labels, "99", "Unused");
    assert_eq!(labels, LIST_KEYS);
}
