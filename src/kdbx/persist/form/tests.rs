//! Unit tests for edge paths in `kdbx::persist::form`.

use std::path::PathBuf;

use super::*;
use crate::kdbx::tests_support::unlocked_sample;

#[test]
fn cancel_edit_while_locked_and_missing_form_error_are_noops() {
    let mut locked = KdbxSession::new(PathBuf::from("locked.kdbx"));
    locked.cancel_edit(Instant::now());
    assert!(locked.state.is_locked());

    set_form_error(None, "ignored".to_owned());
}

#[test]
fn finish_save_clears_form_even_if_catalog_reload_fails() {
    let (_dir, mut session, now) = unlocked_sample();
    session.begin_edit(now).expect("open edit form");
    let id = session
        .vault()
        .expect("unlocked")
        .selected_entry()
        .expect("entry")
        .id;
    std::fs::write(&session.path, b"corrupt").expect("corrupt vault");
    session.finish_save(id, now);
    assert!(session.vault().expect("unlocked").form().is_none());
}
