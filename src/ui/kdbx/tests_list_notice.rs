//! Purpose: A notice on the vault is shown under the entries of the list view.

use super::tests::draw;
use crate::kdbx::tests_support::unlocked_sample;

#[test]
fn a_notice_on_the_vault_is_shown_under_the_entries() {
    let (_dir, mut session, _now) = unlocked_sample();
    let vault = session.vault_mut().expect("an unlocked vault");
    vault.toggle_structure();
    vault.notice = Some("Copied to the clipboard".to_owned());
    let screen = draw(&session);
    assert!(screen.contains("Copied to the clipboard"), "{screen}");
}

#[test]
fn a_notice_on_the_vault_is_shown_in_the_tree_too() {
    let (_dir, mut session, _now) = unlocked_sample();
    session.vault_mut().expect("an unlocked vault").notice =
        Some("Copied to the clipboard".to_owned());
    let screen = draw(&session);
    assert!(screen.contains("Copied to the clipboard"), "{screen}");
}
