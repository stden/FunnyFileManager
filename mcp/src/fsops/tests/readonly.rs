//! Purpose: Pins that `apply_readonly` toggles only the owner's write bit.

use super::mode_of;
use crate::fsops::apply_readonly;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn clearing_readonly_does_not_widen_to_group_or_world() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("key");
    fs::write(&file, "private").expect("write");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("chmod");

    apply_readonly(&file, false).expect("clear readonly");
    assert_eq!(
        mode_of(&file),
        0o600,
        "owner write stays, group/world must not gain it"
    );
}

#[cfg(unix)]
#[test]
fn setting_readonly_clears_every_write_bit() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("shared");
    fs::write(&file, "data").expect("write");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o666)).expect("chmod");

    apply_readonly(&file, true).expect("set readonly");
    assert_eq!(mode_of(&file), 0o444);

    apply_readonly(&file, false).expect("clear readonly");
    assert_eq!(mode_of(&file), 0o644, "only the owner bit comes back");
}
