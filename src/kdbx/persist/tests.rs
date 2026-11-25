//! Unit tests for `close_dialog_if_unlocked` across locked and unlocked states.

use std::path::PathBuf;

use super::*;
use crate::kdbx::tests_support::unlocked_sample;

#[test]
fn close_dialog_if_unlocked_handles_both_session_states() {
    let mut locked = KdbxSession::new(PathBuf::from("locked.kdbx"));
    close_dialog_if_unlocked(&mut locked.state);
    assert!(locked.state.is_locked());

    let (_dir, mut unlocked, _now) = unlocked_sample();
    unlocked.vault_mut().expect("unlocked").open_password();
    close_dialog_if_unlocked(&mut unlocked.state);
    assert!(unlocked.vault().expect("unlocked").is_tree());
}
