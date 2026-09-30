//! Unit tests: a move whose rename the filesystem refuses fails the way `mv`
//! does — the refusal reported, both sides untouched, no copy left behind.
//! Only a rename across filesystems may turn into a copy and a removal.

use super::tests_transfer_policies::{app_between, mark, settings};
use super::*;

use crate::copy::{ExistingFiles, Purpose};

use tempfile::TempDir;

/// Run `transfer` with every rename refused, the way the kernel refuses a
/// user who may not write to the source's folder (`EACCES`, not `EXDEV`).
fn with_renames_denied<T>(transfer: impl FnOnce() -> T) -> T {
    FORCE_RENAME_DENIAL.set(true);
    let result = transfer();
    FORCE_RENAME_DENIAL.set(false);
    result
}

/// `from/{note.txt, dir/inner.txt}`, and an empty `to`.
fn sources() -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::write(from.path().join("note.txt"), b"note").expect("test setup");
    fs::create_dir(from.path().join("dir")).expect("test setup");
    fs::write(from.path().join("dir/inner.txt"), b"inner").expect("test setup");
    let app = app_between(from.path(), to.path());
    (app, from, to)
}

fn assert_untouched(from: &Path, to: &Path, app: &App) {
    assert_eq!(fs::read(from.join("note.txt")).unwrap(), b"note");
    assert_eq!(fs::read(from.join("dir/inner.txt")).unwrap(), b"inner");
    let copies: Vec<_> = fs::read_dir(to).unwrap().collect();
    assert!(copies.is_empty(), "nothing is copied: {copies:?}");
    // The move stops at the first entry it cannot move and says so; the
    // second is not started.
    let status = app.status.to_string();
    assert!(status.contains("Moved 0 item(s), 1 error(s)"), "{status}");
    assert!(status.contains("forced rename denial"), "{status}");
    let crate::models::Overlay::OperationFailed(failure) = &app.overlay else {
        panic!("the failure is shown in a dialog");
    };
    assert_eq!((failure.done(), failure.left()), (0, 1));
    assert!(failure.failed().error().contains("forced rename denial"));
}

#[test]
fn a_refused_rename_in_the_move_dialog_copies_nothing() {
    let (mut app, from, to) = sources();
    mark(&mut app, "note.txt");
    mark(&mut app, "dir");
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;

    with_renames_denied(|| copy_with(&mut app, &dialog));

    assert_untouched(from.path(), to.path(), &app);
}

#[test]
fn a_refused_rename_in_the_quick_move_copies_nothing() {
    let (mut app, from, to) = sources();
    mark(&mut app, "note.txt");
    mark(&mut app, "dir");

    with_renames_denied(|| handle_move(&mut app)).expect("the batch reports");

    assert_untouched(from.path(), to.path(), &app);
}

#[test]
fn a_refused_rename_over_an_existing_file_keeps_both() {
    let (mut app, from, to) = sources();
    fs::write(to.path().join("note.txt"), b"theirs").expect("test setup");
    mark(&mut app, "note.txt");
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    dialog.existing = ExistingFiles::Overwrite;

    with_renames_denied(|| copy_with(&mut app, &dialog));

    assert_eq!(fs::read(from.path().join("note.txt")).unwrap(), b"note");
    assert_eq!(fs::read(to.path().join("note.txt")).unwrap(), b"theirs");
    assert_eq!(fs::read_dir(to.path()).unwrap().count(), 1, "no staging");
    assert!(
        app.status.to_string().contains("1 error(s)"),
        "{}",
        app.status
    );
}

/// The same with a real refusal: nothing may be renamed out of a read-only
/// folder, even by its owner. Root may rename anything, so it skips.
#[cfg(unix)]
#[test]
fn a_rename_out_of_a_read_only_folder_copies_nothing() {
    use std::os::unix::fs::PermissionsExt;

    if crate::platform::is_root() {
        return;
    }
    let (mut app, from, to) = sources();
    mark(&mut app, "note.txt");
    let mut dialog = settings(to.path());
    dialog.purpose = Purpose::Move;
    // Other tests make every rename cross filesystems; not while this runs.
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    fs::set_permissions(from.path(), fs::Permissions::from_mode(0o555)).expect("test setup");
    copy_with(&mut app, &dialog);
    fs::set_permissions(from.path(), fs::Permissions::from_mode(0o755)).expect("cleanup");

    assert_eq!(fs::read(from.path().join("note.txt")).unwrap(), b"note");
    assert_eq!(fs::read_dir(to.path()).unwrap().count(), 0, "no copy");
    assert!(
        app.status.to_string().contains("Permission denied"),
        "{}",
        app.status
    );
}
