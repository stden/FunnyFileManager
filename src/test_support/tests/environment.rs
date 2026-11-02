//! Purpose: Unit tests for editor environment variable lock guards.

use crate::test_support::{env_lock, EditorEnvGuard};

#[test]
fn clear_with_lock_unsets_editor_and_visual_variables() {
    let guard = EditorEnvGuard::clear_with_lock(env_lock());
    assert!(std::env::var_os("EDITOR").is_none());
    assert!(std::env::var_os("VISUAL").is_none());
    drop(guard);
}
