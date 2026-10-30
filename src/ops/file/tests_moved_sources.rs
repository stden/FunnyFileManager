//! Unit tests: a move across filesystems removes from its source only what
//! the copy holds, and reports truthfully what stayed.

use super::remove_moved::{LeftBehind, FORCE_REMOVAL_REFUSAL};
use super::tests_transfer_policies::{app_between, mark, settings};
use super::*;
use crate::copy::Purpose;

use tempfile::TempDir;

/// `from/mixed/{g1.txt, g2.txt, rootsub/r.txt}` with `mixed` marked, and an
/// empty `to`.
fn mixed() -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    let folder = from.path().join("mixed");
    fs::create_dir_all(folder.join("rootsub")).expect("test setup");
    for name in ["g1.txt", "g2.txt", "rootsub/r.txt"] {
        fs::write(folder.join(name), name).expect("test setup");
    }
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "mixed");
    (app, from, to)
}

/// Run `transfer` as a move across filesystems in which `r.txt` will not go,
/// the way the guest could not remove a file of a root-owned subfolder.
fn keeping_r_txt<T>(transfer: impl FnOnce() -> T) -> T {
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    FORCE_REMOVAL_REFUSAL.set(Some("r.txt"));
    let result = transfer();
    FORCE_REMOVAL_REFUSAL.set(None);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    result
}

fn assert_reported_truthfully(app: &App, from: &Path, to: &Path) {
    for name in ["g1.txt", "g2.txt", "rootsub/r.txt"] {
        let copy = to.join("mixed").join(name);
        assert_eq!(
            fs::read(copy).unwrap(),
            name.as_bytes(),
            "the copy is whole"
        );
    }
    assert!(
        from.join("mixed/rootsub/r.txt").is_file(),
        "what would not go"
    );
    assert!(!from.join("mixed/g1.txt").exists(), "the rest went");
    let status = app.status.to_string();
    assert!(
        status.starts_with("Moved 1 item(s), 1 error(s): "),
        "{status}"
    );
    let stayed = "r.txt: the copy is complete, but this stayed in the source";
    assert!(status.contains(stayed), "{status}");
}

#[test]
fn a_dialog_move_across_filesystems_reports_what_stayed() {
    let (mut app, from, to) = mixed();
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;

    keeping_r_txt(|| copy_with(&mut app, &dialog));

    assert_reported_truthfully(&app, from.path(), to.path());
}

#[test]
fn a_quick_move_across_filesystems_reports_what_stayed() {
    let (mut app, from, to) = mixed();

    keeping_r_txt(|| handle_move(&mut app)).expect("the batch reports");

    assert_reported_truthfully(&app, from.path(), to.path());
}

/// Something that appeared in the source while the copy was being made is
/// not in the copy, so the removal must not take it.
#[test]
fn an_entry_the_copy_lacks_is_never_removed() {
    let root = TempDir::new().expect("test setup");
    let (source, copy) = (root.path().join("source"), root.path().join("copy"));
    fs::create_dir_all(source.join("sub")).expect("test setup");
    fs::write(source.join("sub/b.txt"), b"b").expect("test setup");
    crate::utils::copy_dir_all(&source, &copy).expect("test setup");
    fs::write(source.join("sub/later.log"), b"new").expect("test setup");

    let error = remove_moved(&source, &copy).expect_err("something stayed");

    let left = LeftBehind::of(&error).expect("says what stayed");
    assert_eq!(left.path, source.join("sub/later.log"));
    assert_eq!(left.moved_to, copy);
    assert_eq!(fs::read(source.join("sub/later.log")).unwrap(), b"new");
    assert!(!source.join("sub/b.txt").exists(), "what was copied went");
}

/// The same with a real refusal: a read-only subfolder's file cannot be
/// removed by its owner. Root may remove anything, so it skips.
#[cfg(unix)]
#[test]
fn a_read_only_subfolder_stays_behind() {
    use std::os::unix::fs::PermissionsExt;

    if crate::platform::is_root() {
        return;
    }
    let (mut app, from, to) = mixed();
    let locked = from.path().join("mixed/rootsub");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("test setup");
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let moved = handle_move(&mut app);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("cleanup");

    moved.expect("the batch reports");
    assert_reported_truthfully(&app, from.path(), to.path());
}

#[test]
fn a_readonly_file_in_moved_source_is_removed() {
    let root = TempDir::new().expect("test setup");
    let (source, copy) = (root.path().join("source"), root.path().join("copy"));
    fs::create_dir_all(&source).expect("test setup");
    let ro = source.join("ro.txt");
    fs::write(&ro, b"ro").expect("test setup");
    crate::utils::copy_dir_all(&source, &copy).expect("test setup");
    let mut perms = fs::metadata(&ro).expect("meta").permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro, perms).expect("set readonly");

    remove_moved(&source, &copy).expect("read-only moved source removed");
    assert!(!source.exists());
    assert_eq!(fs::read(copy.join("ro.txt")).expect("copy"), b"ro");
}
