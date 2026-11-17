//! Purpose: The walk is safe — symlinks are not followed, an unreadable
//! subdirectory does not fail it, and a walk that saw everything says so.

use super::*;

/// A symlinked loop must not be followed. `a -> ..` is a directory whose
/// contents include itself, and walking it never ends.
#[cfg(unix)]
#[test]
fn a_symlink_loop_does_not_hang_the_walk() {
    let dir = tree(&[("real/file.bin", 42)]);
    std::os::unix::fs::symlink(dir.path(), dir.path().join("real/loop")).expect("symlink");

    let stats = analyse(dir.path());
    assert_eq!(
        stats.total.bytes(),
        42,
        "the link contributes nothing and is not descended into"
    );
}

/// A subdirectory that turns out to be unreadable once the walk descends into
/// it — not merely absent, as the top-level walk already covers — must
/// contribute nothing and leave the rest of the tree intact, the same
/// never-fails contract `analyse` itself gives for its own top level.
#[cfg(unix)]
#[test]
fn a_subdirectory_that_cannot_be_read_contributes_nothing_but_does_not_fail_the_walk() {
    let dir = tree(&[("visible/a.bin", 10)]);
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).expect("test setup");
    fs::write(locked.join("secret.bin"), vec![b'x'; 500]).expect("test setup");
    let _locked = crate::test_support::ModeGuard::set(&locked, 0o000).expect("test setup");
    // Mode 000 stops nobody who is root, and a CI gate often is: then there
    // is no unreadable directory to test, so there is nothing to assert.
    let privileged = fs::read_dir(&locked).is_ok();

    let stats = analyse(dir.path());

    if privileged {
        eprintln!("skipped: this process reads mode-000 directories (root)");
        return;
    }

    assert_eq!(
        child(&stats, "locked").map(|c| c.size.bytes()),
        Some(0),
        "an unreadable subdirectory must contribute nothing rather than failing the walk"
    );
    assert_eq!(
        stats.total.bytes(),
        10,
        "the readable subtree is still counted in full"
    );
}

/// A symlink to a file is not counted either: its target is already counted
/// wherever it really lives, and counting both inflates the total.
#[cfg(unix)]
#[test]
fn a_symlinked_file_is_not_counted_twice() {
    let dir = tree(&[("real.bin", 500)]);
    std::os::unix::fs::symlink(dir.path().join("real.bin"), dir.path().join("alias.bin"))
        .expect("symlink");

    assert_eq!(analyse(dir.path()).total.bytes(), 500);
}

/// A complete walk says so, which is what lets the caller call the numbers
/// totals rather than lower bounds.
#[test]
fn a_small_tree_is_reported_as_completely_measured() {
    let dir = tree(&[("a.bin", 1)]);
    assert!(analyse(dir.path()).coverage.is_complete());
    assert!(!matches!(analyse(dir.path()).coverage, Coverage::Partial));
}
