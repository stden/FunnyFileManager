//! Purpose: Pins `move_entry` on a real second filesystem, and the rule of
//! `remove_moved` that only what the copy holds leaves the source.

use crate::fsops::{copy_entry, remove_moved, Moved, Purpose, FORCE_REMOVAL_REFUSAL};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

/// `<base>/tree/{a.txt, sub/b.txt}`, its files and folders stamped `old`.
fn tree_under(base: &Path, old: SystemTime) -> std::path::PathBuf {
    let tree = base.join("tree");
    fs::create_dir_all(tree.join("sub")).expect("mkdir");
    fs::write(tree.join("a.txt"), "a").expect("write");
    fs::write(tree.join("sub/b.txt"), "b").expect("write");
    for path in ["a.txt", "sub/b.txt", "sub"] {
        let handle = crate::fsops::open_folder(&tree.join(path)).expect("open");
        let times = fs::FileTimes::new().set_modified(old);
        handle.set_times(times).expect("set modification time");
    }
    tree
}

/// Two folders on two filesystems (`/dev/shm` and `/tmp`), or `None` on a
/// machine where they are one — the kernel then never answers `EXDEV`.
#[cfg(unix)]
fn two_filesystems() -> Option<(TempDir, TempDir)> {
    use std::os::unix::fs::MetadataExt;

    let from = TempDir::new_in("/tmp").ok()?;
    let to = TempDir::new_in("/dev/shm").ok()?;
    let device = |dir: &TempDir| fs::metadata(dir.path()).map(|meta| meta.dev()).ok();
    (device(&from)? != device(&to)?).then_some((from, to))
}

/// No seam: the real `EXDEV`, the real copy, the real removal.
#[cfg(unix)]
#[test]
fn a_move_between_two_filesystems_copies_then_removes() {
    use crate::fsops::move_entry;

    let Some((from, to)) = two_filesystems() else {
        return;
    };
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let source = tree_under(from.path(), old);
    let dest = to.path().join("tree");

    let moved = move_entry(&source, &dest).expect("the move lands");

    assert!(matches!(moved, Moved::Whole), "nothing stayed");
    assert!(fs::symlink_metadata(&source).is_err(), "the source is gone");
    assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "a");
    assert_eq!(fs::read_to_string(dest.join("sub/b.txt")).unwrap(), "b");
    for path in ["a.txt", "sub/b.txt", "sub"] {
        let modified = fs::metadata(dest.join(path)).unwrap().modified().unwrap();
        assert_eq!(modified, old, "{path} keeps its modification time");
    }
}

/// Something that appeared in the source while the copy was being made is
/// not in the copy, so the removal must not take it.
#[test]
fn an_entry_the_copy_lacks_is_never_removed() {
    let root = TempDir::new().expect("temp dir");
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let source = tree_under(root.path(), old);
    let copy = root.path().join("copy");
    copy_entry(&source, &copy, Purpose::Move).expect("the copy lands");
    fs::write(source.join("sub/later.log"), "new").expect("write");

    let Moved::Leaving(left) = remove_moved(&source, &copy) else {
        panic!("something stayed");
    };

    assert_eq!(left.path, source.join("sub/later.log"));
    assert_eq!(
        fs::read_to_string(source.join("sub/later.log")).unwrap(),
        "new"
    );
    assert!(!source.join("sub/b.txt").exists(), "what was copied went");
    assert!(!source.join("a.txt").exists(), "what was copied went");
    let report = left.to_string();
    assert!(
        report.contains("the copy is complete, but this stayed in the source"),
        "{report}"
    );
    assert!(report.contains("later.log"), "{report}");
}

/// An entry that will not go stays, everything else still goes, and the
/// first one kept is the one reported.
#[test]
fn what_will_not_go_is_reported_beside_the_copy() {
    let root = TempDir::new().expect("temp dir");
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let source = tree_under(root.path(), old);
    let copy = root.path().join("copy");
    copy_entry(&source, &copy, Purpose::Move).expect("the copy lands");

    FORCE_REMOVAL_REFUSAL.set(Some("b.txt"));
    let moved = remove_moved(&source, &copy);
    FORCE_REMOVAL_REFUSAL.set(None);

    let Moved::Leaving(left) = moved else {
        panic!("b.txt stayed");
    };
    assert_eq!(left.path, source.join("sub/b.txt"));
    assert!(source.join("sub/b.txt").is_file(), "what would not go");
    assert!(!source.join("a.txt").exists(), "the rest went");
    assert!(
        fs::read_to_string(copy.join("sub/b.txt")).is_ok(),
        "the copy is untouched"
    );
}
