//! Purpose: Direct unit tests for folded sort keys and out-of-range index guards.

use std::cmp::Ordering;

use super::{cmp_at, fold, place};
use crate::models::DirEntry;

#[test]
fn folded_keys_implement_eq_and_partial_ord_across_ascii_and_unicode() {
    assert!(fold("Readme.MD") == fold("readme.md"));
    assert!(fold("Äpfel") == fold("äpfel"));
    assert_eq!(fold("a").partial_cmp(&fold("B")), Some(Ordering::Less));
}

#[test]
fn cmp_at_and_place_guard_out_of_bounds_indices() {
    assert_eq!(cmp_at::<()>(&[], &[], &[], 2, 1), Ordering::Greater);
    let mut entries = vec![DirEntry {
        name: "a".to_owned(),
        ..DirEntry::default()
    }];
    place(&mut entries, &[99]);
    assert_eq!(entries[0].name, "");
}
