//! Purpose: A narrowed mode comes back when its guard drops, even from a test
//! that failed — or the `TempDir` above it could never be removed.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;

use crate::test_support::ModeGuard;

#[test]
fn a_mode_guard_gives_the_mode_back_when_the_test_panics() {
    let dir = TempDir::new().expect("dir");
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).expect("locked");
    fs::write(locked.join("inside.txt"), "x").expect("file");
    let original = fs::metadata(&locked).expect("metadata").permissions();

    let unwound = std::panic::catch_unwind(|| {
        let guard = ModeGuard::set(&locked, 0o000).expect("chmod");
        let locked_mode = fs::metadata(&guard).expect("metadata").permissions();
        assert_eq!(locked_mode.mode() & 0o777, 0o000);
        panic!("a failed assertion while the directory is locked");
    });

    assert!(unwound.is_err());
    assert_eq!(
        fs::metadata(&locked).expect("metadata").permissions(),
        original
    );
}

#[test]
fn a_mode_guard_on_a_missing_path_is_an_error() {
    let dir = TempDir::new().expect("dir");
    assert!(ModeGuard::set(&dir.path().join("missing"), 0o000).is_err());
}
