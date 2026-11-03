//! Confirmed deletes: the reloaded summaries drop the entry, and a vault file
//! that went away is reported as a `KeePass` error.

use super::*;

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
    s.access = crate::kdbx::Access::View;
    assert!(matches!(s.confirm_delete(now), Err(KdbxError::ReadOnly)));
    s.access = crate::kdbx::Access::Edit;
    fs::remove_file(&s.path).expect("delete the vault file");
    assert!(matches!(s.confirm_delete(now), Err(KdbxError::Keepass(_))));
    assert_eq!(KdbxError::NoEdit.to_string(), "No entry is being edited");
}

#[test]
fn undo_last_change_restores_deleted_entry_and_clears_undo_slot() {
    let (_dir, mut session, now) = unlocked_sample();
    assert!(matches!(
        session.undo_last_change(now),
        Err(KdbxError::NothingToUndo)
    ));
    focus(&mut session, "GitHub", now);
    session.request_delete(now).unwrap();
    session.confirm_delete(now).unwrap();
    session.vault_mut().unwrap().set_filter("", now);
    assert!(!sorted_visible_titles(&session).contains(&"GitHub".to_owned()));
    session.undo_last_change(now).unwrap();
    assert!(sorted_visible_titles(&session).contains(&"GitHub".to_owned()));
    assert!(matches!(
        session.undo_last_change(now),
        Err(KdbxError::NothingToUndo)
    ));
}
