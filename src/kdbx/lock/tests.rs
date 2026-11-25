//! Unit tests for `KdbxSession::lock` and `touch` when already locked.

use std::path::PathBuf;

use super::*;

#[test]
fn lock_and_touch_are_safe_noops_when_already_locked() {
    let mut locked = KdbxSession::new(PathBuf::from("locked.kdbx"));
    locked.lock();
    locked.touch(Instant::now());
    assert!(locked.state.is_locked());
}
