//! Purpose: A move within one filesystem stays a rename, and a transfer onto a link to itself is refused.

use super::*;

// ─── the move must stay a rename ────────────────────────────────────────────

/// Moving inside one filesystem must rename, not copy.
///
/// A rename changes a directory entry and leaves the data alone: measured at
/// four microseconds for five hundred files, against seven milliseconds to
/// copy and delete the same ones — over a thousand times the cost, and the
/// program is frozen for all of it because transfers run on the event loop.
///
/// A timing test would be flaky, so the property is asserted structurally: the
/// destination has the same inode as the source did. A copy cannot produce
/// that, so this fails the moment the fast path is lost.
#[cfg(unix)]
#[test]
fn a_move_within_one_filesystem_keeps_the_same_inode() {
    use std::os::unix::fs::MetadataExt;

    // `rename_or_fail` consults a process-wide flag that other tests toggle to
    // force the cross-device path. Without this lock those tests make the fast
    // path unavailable here, and the inode changes for a reason that has
    // nothing to do with the code under test.
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let temp = TempDir::new().expect("temp dir");
    let source_dir = temp.path().join("from");
    let target_dir = temp.path().join("to");
    fs::create_dir_all(&source_dir).expect("source dir");
    fs::create_dir_all(&target_dir).expect("target dir");

    let source = source_dir.join("payload.bin");
    fs::write(&source, vec![b'x'; 64 * 1024]).expect("payload");
    let before = fs::metadata(&source).expect("before").ino();

    let moved = move_into(&source, &target_dir).expect("move");

    assert!(!source.exists(), "the original is gone");
    let after = fs::metadata(&moved).expect("after").ino();
    assert_eq!(
        before, after,
        "a move within one filesystem must rename rather than copy the bytes"
    );
}

/// The same for a whole directory: a tree of a thousand files must move by
/// one `rename`, not by walking it twice.
#[cfg(unix)]
#[test]
fn moving_a_directory_within_one_filesystem_keeps_the_same_inode() {
    use std::os::unix::fs::MetadataExt;

    // `rename_or_fail` consults a process-wide flag that other tests toggle to
    // force the cross-device path. Without this lock those tests make the fast
    // path unavailable here, and the inode changes for a reason that has
    // nothing to do with the code under test.
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let temp = TempDir::new().expect("temp dir");
    let source_dir = temp.path().join("from");
    let target_dir = temp.path().join("to");
    let tree = source_dir.join("tree");
    fs::create_dir_all(tree.join("nested")).expect("tree");
    fs::create_dir_all(&target_dir).expect("target dir");
    for index in 0..20 {
        fs::write(tree.join(format!("file-{index}.txt")), b"payload").expect("seed");
    }

    let before = fs::metadata(&tree).expect("before").ino();
    let moved = move_into(&tree, &target_dir).expect("move");

    assert!(!tree.exists(), "the original is gone");
    assert_eq!(
        before,
        fs::metadata(&moved).expect("after").ino(),
        "a directory move within one filesystem must rename the directory"
    );
    assert!(moved.join("nested").is_dir(), "and carry its contents");
    assert_eq!(fs::read_dir(&moved).expect("listing").count(), 21);
}

/// The F6 dialog takes its own path through `copy_with`, so it needs the same
/// guarantee — it would be easy to lose it in one and not the other.
#[cfg(unix)]
#[test]
fn the_move_dialog_also_renames_rather_than_copying() {
    use std::os::unix::fs::MetadataExt;

    use crate::copy::{CopyPrompt, Purpose};

    // `rename_or_fail` consults a process-wide flag that other tests toggle to
    // force the cross-device path. Without this lock those tests make the fast
    // path unavailable here, and the inode changes for a reason that has
    // nothing to do with the code under test.
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let temp = TempDir::new().expect("temp dir");
    let source_dir = temp.path().join("from");
    let target_dir = temp.path().join("to");
    fs::create_dir_all(&source_dir).expect("source dir");
    fs::create_dir_all(&target_dir).expect("target dir");
    let source = source_dir.join("payload.bin");
    fs::write(&source, vec![b'x'; 64 * 1024]).expect("payload");
    let before = fs::metadata(&source).expect("before").ino();

    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(source_dir).expect("panel");
    app.right_panel = Panel::new(target_dir.clone()).expect("panel");
    assert!(app.left_panel.select_named("payload.bin"));

    let settings = CopyPrompt::for_purpose(Purpose::Move, "\"payload.bin\"", &target_dir);
    copy_with(&mut app, &settings);

    let moved = target_dir.join("payload.bin");
    assert!(moved.exists(), "it arrived");
    assert!(!source.exists(), "and it left");
    assert_eq!(
        before,
        fs::metadata(&moved).expect("after").ino(),
        "F6 must rename too, not copy and delete"
    );
}

#[test]
fn delete_without_confirmation_removes_the_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("gone.txt"), "x")?;
    let mut app = app_with_panels(temp.path(), temp.path())?;
    assert!(app.get_active_panel().select_named("gone.txt"));
    app.settings
        .set(crate::menu::SettingsPage::Confirmations, "off")
        .expect("setting");
    handle_delete_confirm(&mut app)?;
    assert!(!temp.path().join("gone.txt").exists());
    Ok(())
}

#[test]
fn a_transfer_onto_a_symlink_of_the_same_file_is_refused() {
    let temp = TempDir::new().expect("temp");
    let file = temp.path().join("a.txt");
    fs::write(&file, "x").expect("file");
    let link = temp.path().join("b.txt");
    symlink(&file, &link).expect("link");
    assert!(reject_transfer_onto_itself(&file, &link).is_err());
}
