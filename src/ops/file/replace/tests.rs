//! Purpose: Atomic entry replacement keeps sources and link targets intact on failure.

use super::*;
use tempfile::TempDir;

fn files() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = TempDir::new().unwrap();
    let source = root.path().join("source");
    let dest = root.path().join("dest");
    fs::write(&source, b"new").unwrap();
    fs::write(&dest, b"old").unwrap();
    (root, source, dest)
}

#[test]
fn a_successful_overwrite_leaves_no_staging_entry() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (root, source, dest) = files();
        let prompt = CopyPrompt::for_purpose(purpose, "source", root.path());
        entry(&source, &dest, &prompt).unwrap();
        assert_eq!(fs::read(dest).unwrap(), b"new");
        assert_eq!(source.exists(), purpose == Purpose::Copy);
        let count = if purpose == Purpose::Copy { 2 } else { 1 };
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), count);
    }
}

#[test]
fn a_destination_that_became_a_folder_preserves_the_source_and_cleans_staging() {
    for purpose in [Purpose::Copy, Purpose::Move] {
        let (root, source, dest) = files();
        fs::remove_file(&dest).unwrap();
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("precious"), b"old").unwrap();
        let prompt = CopyPrompt::for_purpose(purpose, "source", root.path());
        assert!(entry(&source, &dest, &prompt).is_err());
        assert_eq!(fs::read(source).unwrap(), b"new");
        assert_eq!(fs::read(dest.join("precious")).unwrap(), b"old");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

#[cfg(unix)]
#[test]
fn overwriting_a_symlink_replaces_the_link_and_preserves_its_target() {
    let (root, source, dest) = files();
    let target = root.path().join("target");
    fs::rename(&dest, &target).unwrap();
    std::os::unix::fs::symlink(&target, &dest).unwrap();

    entry(&source, &dest, &CopyPrompt::new("source", root.path())).unwrap();

    assert!(!fs::symlink_metadata(&dest).unwrap().is_symlink());
    assert_eq!(fs::read(dest).unwrap(), b"new");
    assert_eq!(fs::read(target).unwrap(), b"old");
}

#[cfg(unix)]
#[test]
fn staged_relative_links_keep_their_target_when_copying_timestamps() {
    let (root, source, dest) = files();
    let link = root.path().join("link");
    std::os::unix::fs::symlink("source", &link).unwrap();
    let mut prompt = CopyPrompt::new("link", root.path());
    prompt.timestamps = true;

    entry(&link, &dest, &prompt).unwrap();

    assert_eq!(fs::read_link(&dest).unwrap(), Path::new("source"));
    assert_eq!(fs::read(dest).unwrap(), fs::read(source).unwrap());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
}

#[test]
fn a_move_fallback_replaces_the_destination_before_removing_the_source() {
    use super::super::{FORCE_RENAME_FAILURE, RENAME_TEST_MUTEX};
    use std::sync::atomic::Ordering;

    let _guard = RENAME_TEST_MUTEX.lock().unwrap();
    let (root, source, dest) = files();
    let prompt = CopyPrompt::for_purpose(Purpose::Move, "source", root.path());
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let result = entry(&source, &dest, &prompt);
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);

    result.unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(dest).unwrap(), b"new");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn overwriting_keeps_the_readonly_attribute_copied_from_the_source() {
    let (root, source, dest) = files();
    let writable = fs::metadata(&source).unwrap().permissions();
    let mut readonly = writable.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&source, readonly).unwrap();

    entry(&source, &dest, &CopyPrompt::new("source", root.path())).unwrap();

    assert!(fs::metadata(&dest).unwrap().permissions().readonly());
    fs::set_permissions(source, writable.clone()).unwrap();
    fs::set_permissions(dest, writable).unwrap();
}

#[cfg(unix)]
mod tests_in_place;
