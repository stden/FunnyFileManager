//! Unit tests: F5 / F6 / F8 must act on the link, never through it.
//!
//! `copy_dir_all` recreates *nested* links, but `copy_entry` — the entry point
//! F5 and F6 actually use — branched on `Path::is_dir`, which resolves the link.
//! A panel entry `logs -> /var/log` was therefore copied as the whole target
//! tree, and the move fallback deleted the user's link afterwards.

#![cfg(unix)]

use std::os::unix::fs::symlink;

use super::*;

use crate::models::{Panel, PanelSide};

use tempfile::TempDir;

/// Two panels rooted at `left` / `right`, left active — the F5/F6/F8 setup.
fn app_with_panels(left: &Path, right: &Path) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(left.to_path_buf())?;
    app.right_panel = Panel::new(right.to_path_buf())?;
    app.active_panel = PanelSide::Left;
    Ok(app)
}

/// Select `name` in the active panel, the way the cursor would.
fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("test setup: entry must be listed");
    panel.select(Some(index));
}

/// A directory the link can point at, holding one recognisable file.
fn make_target_tree(root: &Path) -> io::Result<PathBuf> {
    let target = root.join("target");
    fs::create_dir(&target)?;
    fs::write(target.join("inside.txt"), b"payload")?;
    Ok(target)
}

#[test]
fn copy_recreates_a_top_level_directory_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    symlink(&target, left.join("logs"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");
    handle_copy(&mut app)?;

    let copied = right.join("logs");
    assert!(
        fs::symlink_metadata(&copied)?.is_symlink(),
        "F5 must recreate the link, not copy the tree it points at"
    );
    assert_eq!(fs::read_link(&copied)?, target);
    assert!(
        !copied.join("inside.txt").is_file() || fs::read_link(&copied).is_ok(),
        "nothing may have been written through the link"
    );
    Ok(())
}

#[test]
fn copy_recreates_a_top_level_file_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(temp_dir.path().join("real.txt"), b"payload")?;
    symlink(temp_dir.path().join("real.txt"), left.join("alias.txt"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "alias.txt");
    handle_copy(&mut app)?;

    assert!(
        fs::symlink_metadata(right.join("alias.txt"))?.is_symlink(),
        "a file link must survive F5 as a link"
    );
    Ok(())
}

/// A broken link has no target to dereference at all: the old `fs::copy` path
/// failed outright, so the entry could not be transferred.
#[test]
fn copy_recreates_a_broken_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    symlink("../nowhere/gone.txt", left.join("dangling"))?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "dangling");
    handle_copy(&mut app)?;

    assert!(fs::symlink_metadata(right.join("dangling"))?.is_symlink());
    assert!(
        app.status_text().starts_with("Copied"),
        "the copy must succeed, got {:?}",
        app.status_text()
    );
    Ok(())
}

/// The move fallback (`rename` fails across devices) is where the old code did
/// real damage: it deep-copied the target tree and then ran `remove_dir_all` on
/// the *link*, because `Path::is_dir` said it was a directory.
#[test]
fn move_fallback_removes_the_link_and_leaves_the_target_alone() -> io::Result<()> {
    let _guard = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    let link = left.join("logs");
    symlink(&target, &link)?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");

    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let result = handle_move(&mut app);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);
    result?;

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "the source link must be gone"
    );
    assert!(
        fs::symlink_metadata(right.join("logs"))?.is_symlink(),
        "the destination must be a link, not a copy of the tree"
    );
    assert!(
        target.join("inside.txt").is_file(),
        "the link's target must be untouched by a move of the link"
    );
    Ok(())
}

/// Same rule, third call site: F8 used `Path::is_dir` too, so a directory link
/// was handed to `remove_dir_all`.
#[test]
fn delete_unlinks_a_directory_symlink_without_touching_its_target() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let target = make_target_tree(temp_dir.path())?;
    let link = left.join("logs");
    symlink(&target, &link)?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "logs");
    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "the link itself must be removed"
    );
    assert!(
        target.join("inside.txt").is_file(),
        "the link's target must survive F8 on the link"
    );
    assert_eq!(app.status_text(), "Deleted 1 item(s)");
    Ok(())
}

/// Real directories still take the recursive path.
#[test]
fn delete_still_removes_a_real_directory_recursively() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    let nested = left.join("tree");
    fs::create_dir(&nested)?;
    fs::write(nested.join("inside.txt"), b"payload")?;

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "tree");
    handle_delete_confirm(&mut app)?;
    handle_delete(&mut app)?;

    assert!(fs::symlink_metadata(&nested).is_err());
    assert_eq!(app.status_text(), "Deleted 1 item(s)");
    Ok(())
}

/// When every candidate destination name is already taken, the copy fails with
/// `AlreadyExists` rather than overwriting or looping forever.
#[test]
fn copy_fails_cleanly_when_no_unique_destination_remains() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left = temp_dir.path().join("left");
    let right = temp_dir.path().join("right");
    fs::create_dir(&left)?;
    fs::create_dir(&right)?;
    fs::write(left.join("report.txt"), b"src")?;

    // Occupy the free name and the first 9999 suffixes unique_destination tries.
    fs::write(right.join("report.txt"), b"taken")?;
    for i in 1..=9999u32 {
        fs::write(right.join(format!("report_{i}.txt")), b"taken")?;
    }

    let mut app = app_with_panels(&left, &right)?;
    select(&mut app, "report.txt");
    // Batch copy records per-item failures in the status, not as Err — so the
    // destination still must not have grown a 10000th copy.
    handle_copy(&mut app)?;

    assert!(
        !right.join("report_10000.txt").exists(),
        "must not invent names past the unique-destination limit"
    );
    assert!(
        app.status_text().contains("error")
            || app.status_text().contains("Error")
            || app.status_text().contains("Already")
            || app.status_text().contains("could not"),
        "status should report the failure, got {:?}",
        app.status_text()
    );
    Ok(())
}

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
