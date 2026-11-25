//! Unit tests for `Vault` dialog transitions when already showing a dialog.

use crate::kdbx::tests_support::unlocked_sample;
use crate::kdbx::vault::{BrowsePage, Page};

#[test]
fn open_password_and_request_delete_ignore_non_browse_pages() {
    let (_dir, mut session, _now) = unlocked_sample();
    let vault = session.vault_mut().expect("unlocked");

    vault.open_password();
    vault.open_password();
    assert_eq!(vault.page, Page::Password(BrowsePage::Tree));

    vault.close_dialog();
    vault.request_delete();
    vault.request_delete();
    assert_eq!(vault.page, Page::DeleteConfirm(BrowsePage::Tree));
}
