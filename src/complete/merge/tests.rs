//! Purpose: Completion sources merge without changing first-seen order.

use super::append_unique;

#[test]
fn merge_preserves_source_order_and_removes_incoming_duplicates() {
    let existing = ["builtin", "shared", "shared"].map(str::to_owned).into();
    let incoming = ["shared", "new", "new", "Builtin", "last"]
        .map(str::to_owned)
        .into();
    assert_eq!(
        append_unique(existing, incoming),
        ["builtin", "shared", "shared", "new", "Builtin", "last"]
    );
}

#[test]
fn merging_large_overlapping_sources_matches_first_seen_order() {
    let existing = (0..10_000).map(|i| format!("file-{i}")).collect();
    let incoming = (5_000..15_000).map(|i| format!("file-{i}")).collect();
    let expected: Vec<_> = (0..15_000).map(|i| format!("file-{i}")).collect();
    assert_eq!(append_unique(existing, incoming), expected);
    assert_eq!(append_unique(Vec::new(), Vec::new()), Vec::<String>::new());
}
