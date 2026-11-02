//! Edit-form tests: saves round-trip through the real file, and a failed or
//! refused save keeps the draft with a secret-free error.

use super::*;

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
