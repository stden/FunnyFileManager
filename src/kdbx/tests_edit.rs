//! Edit-form tests: `F4` loads the selected entry, keystrokes land in the
//! focused field, saves round-trip through the real file, and a failed save
//! keeps the draft with a secret-free error.

use std::fs;

use super::tests_support::{
    focus, selected_title, sorted_visible_titles, type_password, unlocked_sample, VAULT_PASSWORD,
};
use super::{EditField, EntryForm, FormMode, KdbxError, KdbxSession};

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
fn typing_goes_to_the_focused_field_and_sets_dirty() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");

    session.type_char('x', now);
    {
        let vault = session.vault().expect("vault is unlocked");
        let form = vault.form().expect("form is open");
        assert_eq!(form.title(), "GitHubx");
        assert!(form.is_dirty());
        assert_eq!(
            vault.filter(),
            "GitHub",
            "the keystroke went to the form, not the filter"
        );
    }

    session.backspace(now);
    assert_eq!(open_form(&session).title(), "GitHub");
    assert!(
        open_form(&session).is_dirty(),
        "an undone edit is still an edit"
    );

    open_form_mut(&mut session).focus_next();
    assert_eq!(open_form(&session).focus(), EditField::Username);
    session.type_char('!', now);
    assert_eq!(open_form(&session).username(), "alice!");
}

#[test]
fn password_keystrokes_stay_in_the_focused_secret_field() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    open_form_mut(&mut session).focus_next();
    // Two steps back from Username wraps Title -> Password: a ring, like the
    // name dialog's tab order.
    open_form_mut(&mut session).focus_prev();
    open_form_mut(&mut session).focus_prev();
    assert_eq!(open_form(&session).focus(), EditField::Password);
    session.type_char('z', now);
    assert_eq!(open_form(&session).password_masked(), "•".repeat(6));
    session.backspace(now);
    assert_eq!(open_form(&session).password_masked(), "•".repeat(5));
    assert_eq!(
        open_form(&session).title(),
        "GitHub",
        "password keystrokes stay out of the text fields"
    );
}

#[test]
fn save_edit_persists_to_disk() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "bank", now);
    session.begin_edit(now).expect("begin edit");

    open_form_mut(&mut session).focus_next(); // Title -> Username
    for ch in "xyz".chars() {
        session.type_char(ch, now);
    }
    open_form_mut(&mut session).focus_prev(); // Username -> Title
    open_form_mut(&mut session).focus_prev(); // Title -> Password (ring)
    session.type_char('!', now);

    assert!(session.save_edit(now), "save must succeed");
    assert!(
        session.vault().expect("vault is unlocked").form().is_none(),
        "the form closes on success"
    );

    // A fresh session sees the new values only if they reached the file.
    let mut fresh = KdbxSession::new(session.path.clone());
    type_password(&mut fresh, VAULT_PASSWORD, now);
    assert!(
        fresh.unlock(now),
        "the vault must still unlock after the save"
    );
    focus(&mut fresh, "bank", now);
    assert_eq!(fresh.copy_username().expect("copy"), "carolxyz");
    assert_eq!(fresh.copy_password(now).expect("copy"), "bk-pw!");
}

#[test]
fn save_edit_refreshes_summaries_and_reselects_by_title() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    // Clear the filter: after the rename the old title matches nothing, so the
    // reselect-by-identity below would have no row to land on.
    session
        .vault_mut()
        .expect("vault is unlocked")
        .set_filter("", now);

    for ch in "-2".chars() {
        session.type_char(ch, now);
    }
    assert!(session.save_edit(now), "rename must succeed");

    let titles = sorted_visible_titles(&session);
    assert!(
        titles.contains(&"GitHub-2".to_owned()),
        "the summaries show the new title"
    );
    assert!(
        !titles.contains(&"GitHub".to_owned()),
        "the rename updated the entry instead of adding a sibling"
    );
    assert_eq!(
        selected_title(&session).as_deref(),
        Some("GitHub-2"),
        "reselected by identity, not by index"
    );
}

#[test]
fn failed_save_keeps_the_form_and_records_a_safe_error() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    session.type_char('x', now);
    fs::remove_file(&session.path).expect("delete the vault file");

    assert!(!session.save_edit(now), "the save must fail");

    let form = open_form(&session);
    assert_eq!(form.title(), "GitHubx", "nothing typed is lost");
    let error = form.error().expect("the failure is recorded on the form");
    for word in VAULT_PASSWORD.split(' ') {
        assert!(
            !error.contains(word),
            "the master password leaked into the error"
        );
    }
    assert!(
        !error.contains("gh-pw"),
        "the entry password leaked into the error"
    );
}

#[test]
fn rename_collision_reports_already_exists_text() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    for _ in "GitHub".chars() {
        session.backspace(now);
    }
    for ch in "Gmail".chars() {
        session.type_char(ch, now);
    }

    assert!(
        !session.save_edit(now),
        "renaming onto an existing sibling must be refused"
    );
    assert_eq!(
        open_form(&session).error(),
        Some("An entry with that title already exists")
    );
}

#[test]
#[allow(unsafe_code)]
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

#[test]
fn zzz_probe_confirm_delete_reloads_catalog() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.request_delete(now).expect("request delete");
    session.confirm_delete(now).expect("confirm delete");
    session
        .vault_mut()
        .expect("vault is unlocked")
        .set_filter("", now);
    let titles = sorted_visible_titles(&session);
    assert!(
        !titles.contains(&"GitHub".to_owned()),
        "delete must drop the title from the reloaded summaries: {titles:?}"
    );
    assert_eq!(titles.len(), 4, "the other four survive: {titles:?}");
}

#[test]
fn zzz_tmp_probe_confirm_delete_keepass_error() {
    let (_d, mut s, now) = unlocked_sample();
    fs::remove_file(&s.path).expect("delete the vault file");
    assert!(matches!(s.confirm_delete(now), Err(KdbxError::Keepass(_))));
}

#[test]
fn zz_probe_undo_without_record_returns_false() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    assert!(!open_form_mut(&mut session).undo());
}

#[test]
fn zz_probe_newline_is_refused() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");

    open_form_mut(&mut session).push_char('\n');
    open_form_mut(&mut session).push_char('\r');
    session.type_char('\n', now);

    assert_eq!(open_form(&session).title(), "GitHub");
    assert!(!open_form(&session).is_dirty());
}

#[test]
fn zz_probe_backspace_on_empty_notes_drops_the_undo() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "Jenkins", now);
    session.begin_edit(now).expect("begin edit");
    let form = open_form_mut(&mut session);
    form.focus_next();
    form.focus_next();
    form.focus_next();
    assert_eq!(form.focus(), EditField::Notes);
    assert_eq!(form.notes(), "");
    form.backspace();
    assert!(!form.is_dirty(), "an empty-field backspace is not an edit");
    assert!(!form.undo(), "the discarded undo leaves nothing to restore");
}

/// A 0444 vault used to be replaced by the save (and left 0600). The save is
/// now refused, and the form says why in words rather than as an I/O error.
/// Root may write a read-only file, so the refusal is only observable without
/// it.
#[cfg(unix)]
#[test]
fn saving_into_a_read_only_vault_is_refused_on_the_form() {
    use std::os::unix::fs::PermissionsExt as _;

    if crate::platform::is_root() {
        return;
    }
    let (_dir, mut session, now) = unlocked_sample();
    fs::set_permissions(&session.path, fs::Permissions::from_mode(0o444)).expect("chmod");
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("begin edit");
    session.type_char('x', now);

    assert!(!session.save_edit(now), "a read-only vault is not replaced");

    let error = open_form(&session).error().expect("the refusal is shown");
    assert!(error.contains("read-only"), "{error}");
    let mode = fs::metadata(&session.path)
        .expect("vault")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o444, "the vault keeps its mode");
}
