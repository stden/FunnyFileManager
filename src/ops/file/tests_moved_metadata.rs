//! Unit tests: a move that has to copy, because the two folders are on
//! different filesystems, keeps what a rename keeps — every entry's times
//! and modes — as `mv` does.

use std::time::SystemTime;

use super::tests_kept_metadata::{mode, modified, tree, TREE};
use super::tests_transfer_policies::settings;
use super::*;
use crate::copy::Purpose;

/// Run `transfer` with every rename failing as it does between two
/// filesystems.
fn across_filesystems<T>(transfer: impl FnOnce() -> T) -> T {
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let result = transfer();
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    result
}

/// The time and mode of every entry of the tree under `root`.
fn snapshot(root: &Path) -> Vec<(&'static str, SystemTime, u32)> {
    TREE.iter()
        .map(|&(name, _, _)| (name, modified(&root.join(name)), mode(&root.join(name))))
        .collect()
}

#[test]
fn a_move_across_filesystems_keeps_every_time_and_mode() {
    let (mut app, from, to) = tree();
    let before = snapshot(from.path());
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;

    across_filesystems(|| copy_with(&mut app, &dialog));

    assert!(!from.path().join("top").exists(), "{}", app.status);
    assert_eq!(snapshot(to.path()), before);
}

#[test]
fn the_quick_move_across_filesystems_keeps_them_too() {
    let (mut app, from, to) = tree();
    let before = snapshot(from.path());

    across_filesystems(|| handle_move(&mut app)).expect("the batch reports");

    assert!(!from.path().join("top").exists(), "{}", app.status);
    assert_eq!(snapshot(to.path()), before);
}
