//! Unit tests for locked no-op paths in `kdbx::browse`.

use std::path::PathBuf;

use super::*;

#[test]
fn move_selection_and_close_vault_dialog_are_noops_while_locked() {
    let mut locked = KdbxSession::new(PathBuf::from("locked.kdbx"));
    let now = Instant::now();
    locked.move_selection(Motion::Next, now);
    locked.close_vault_dialog(now);
    assert!(locked.state.is_locked());
}
