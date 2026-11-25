//! Unit tests for `Vault::select_entry_by_id` when the target entry is filtered out.

use crate::kdbx::tests_support::unlocked_sample;

#[test]
fn select_entry_by_id_keeps_selection_when_entry_is_hidden_by_filter() {
    let (_dir, mut session, now) = unlocked_sample();
    let vault = session.vault_mut().expect("unlocked");
    let hidden_id = vault.entries()[0].id;

    for ch in "zzzznomatch".chars() {
        vault.push_filter_char(ch, now);
    }
    let empty: &[usize] = &[];
    assert_eq!(vault.matches(), empty);
    vault.select_entry_by_id(hidden_id);
    assert_eq!(vault.selected(), 0);
}
