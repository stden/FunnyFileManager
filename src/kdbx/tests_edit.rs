//! Edit-form tests: `F4` loads the selected entry, locking or cancelling drops
//! the draft, and no save without an editable form reaches the database.
//! Keystrokes are in `tests_edit/tests_typing.rs`, saves that
//! round-trip through the real file — and a failed save that keeps the draft
//! with a secret-free error — in `tests_edit/tests_save.rs`, and confirmed
//! deletes in `tests_edit/tests_delete.rs`.

use std::fs;

use super::tests_support::{
    focus, selected_title, sorted_visible_titles, type_password, unlocked_sample, VAULT_PASSWORD,
};
use super::{EditField, EntryForm, FormMode, KdbxError, KdbxSession};

mod tests_delete;
mod tests_save;
mod tests_typing;

/// The open form, or a panic that names the missing precondition.
fn open_form(session: &KdbxSession) -> &EntryForm {
    session
        .vault()
        .expect("vault is unlocked")
        .form()
        .expect("form is open")
}

fn open_form_mut(session: &mut KdbxSession) -> &mut EntryForm {
    session
        .vault_mut()
        .expect("vault is unlocked")
        .form_mut()
        .expect("form is open")
}

#[test]
fn begin_edit_loads_the_selected_nested_entry() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "Jenkins", now);

    session.begin_edit(now).expect("begin edit");

    let form = open_form(&session);
    assert_eq!(form.title(), "Jenkins");
    assert_eq!(form.username(), "ci-bot");
    assert_eq!(form.url(), "https://ci.internal");
    assert_eq!(form.notes(), "");
    // The origin identity is what `save_edit` addresses, so a renamed title
    // still updates this entry instead of upserting a root-group sibling.
    assert_eq!(form.origin_title(), "Jenkins");
    assert_eq!(form.origin_group(), ["Work", "CI"]);
    assert_eq!(form.focus(), EditField::Title);
    assert_eq!(form.mode(), FormMode::Editing);
    assert!(!form.is_dirty());
    assert!(form.error().is_none());

    // Editing is the explicit secret-bearing dialog, so its password is
    // immediately visible and ready to copy.
    assert_eq!(form.password_masked(), "•".repeat("jk-pw".len()));
    assert_eq!(open_form(&session).password_visible(), "jk-pw");
}

#[test]
#[allow(
    unsafe_code,
    reason = "reads the wiped buffer back through a raw pointer to prove its bytes were overwritten"
)]
fn lock_while_editing_drops_and_zeroizes_the_form() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");

    let vault = session.vault_mut().expect("vault is unlocked");
    let (ptr, len) = {
        let secret = vault.form().expect("form is open").password().expose();
        (secret.as_ptr(), secret.len())
    };
    assert!(len > 0, "the fixture entry has a password");

    // Zeroization is asserted on the live vault, as in `tests_secrets`: after
    // `lock` the form is freed and reading its buffer would be use-after-free.
    vault.zeroize();
    let form = vault
        .form()
        .expect("zeroize wipes in place; only lock drops");
    assert!(form.password().expose().is_empty());
    // SAFETY: the form — and so the draft's allocation — is still alive;
    // `zeroize` only reset the length after overwriting the bytes.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    assert!(
        bytes.iter().all(|&byte| byte == 0),
        "the draft password survived zeroize: {bytes:?}"
    );

    session.lock();
    assert!(
        session.vault().is_none(),
        "the form went down with the vault"
    );
    assert!(matches!(session.begin_edit(now), Err(KdbxError::Locked)));
}

#[test]
fn cancel_edit_discards() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    for ch in "-junk".chars() {
        session.type_char(ch, now);
    }

    session.cancel_edit(now);
    assert!(session
        .vault()
        .expect("cancel keeps the vault unlocked")
        .form()
        .is_none());

    // Nothing reached the file: a fresh form starts from the stored values.
    session.begin_edit(now).expect("begin edit again");
    let form = open_form(&session);
    assert_eq!(form.title(), "GitHub");
    assert!(!form.is_dirty());
}

#[test]
fn saving_without_an_editable_form_never_changes_the_database() {
    let (_directory, mut session, now) = unlocked_sample();
    let original = fs::read(&session.path).expect("original database");
    assert!(!session.save_edit(now), "no form is open");
    session.access = super::Access::View;
    assert!(!session.save_edit(now), "view mode refuses writes");
    session.lock();
    session.access = super::Access::Edit;
    assert!(!session.save_edit(now), "locked database refuses writes");
    assert_eq!(
        fs::read(&session.path).expect("unchanged database"),
        original
    );
}
