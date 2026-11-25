//! Unit tests for `VaultTreeLayout` edge cases with unmatched groups and empty `match_row`.

use super::*;
use crate::kdbx::secret::MasterKey;
use crate::kdbx::tests_support::unlocked_sample;
use std::time::Instant;

#[test]
fn tree_layout_skips_entries_with_unknown_group_and_short_match_row() {
    let (_dir, session, _now) = unlocked_sample();
    let orphan = session.vault().expect("unlocked").entries()[0].clone();
    let vault = Vault::new(
        MasterKey::new("pw".to_owned()),
        None,
        vec![orphan],
        Vec::new(),
        Instant::now(),
    );
    let row_count = vault.with_tree_layout(|layout| layout.rows.len());
    assert_eq!(row_count, 0);

    let mut rows = Vec::new();
    append_bucket(&mut rows, &mut [], 0, vec![(0, 0)]);
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0], VaultTreeRow::Entry { .. }));
}
