use super::*;

#[test]
fn copying_a_directory_inside_itself_is_refused_before_claiming_the_destination() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("nested/copy");
    let error = reject_copy_into_itself(root.path(), &destination).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!destination.exists());
    reject_copy_into_itself(&root.path().join("missing"), &destination).unwrap();
}

#[test]
fn a_directory_copy_whose_fill_fails_removes_the_claimed_destination() {
    let root = tempfile::tempdir().unwrap();
    let meta = fs::metadata(root.path()).unwrap();
    let missing_src = root.path().join("vanished");
    let destination = root.path().join("partial_dir");

    let error = copy_dir(&missing_src, &destination, &meta, Purpose::Copy).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!destination.exists());
}

#[cfg(windows)]
#[test]
fn a_locked_source_leaves_no_partial_file_copy() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("locked.bin");
    fs::write(&source, b"locked bytes").unwrap();
    let guard = fs::File::open(&source).unwrap();
    guard.lock().unwrap();

    let destination = root.path().join("copy.bin");
    let result = copy_entry(&source, &destination, Purpose::Copy);
    drop(guard);

    assert!(result.is_err());
    assert!(!destination.exists());
}

#[cfg(windows)]
#[test]
fn windows_symlink_helper_dispatches_both_directory_and_file_links() {
    let root = tempfile::tempdir().unwrap();
    let dir_target = root.path().join("target_dir");
    fs::create_dir(&dir_target).unwrap();
    let junction = root.path().join("junction_src");
    crate::tests_tools::make_dir_link_cross(&dir_target, &junction);

    let dir_dst = root.path().join("copied_dir_link");
    if copy_entry(&junction, &dir_dst, Purpose::Copy).is_ok() {
        assert!(fs::symlink_metadata(&dir_dst).unwrap().is_symlink());
    } else {
        assert!(!dir_dst.exists());
    }

    let file_target = root.path().join("target.txt");
    fs::write(&file_target, b"x").unwrap();
    let file_dst = root.path().join("copied_file_link");
    if create_windows_symlink(&file_target, &file_dst, false).is_ok() {
        assert!(fs::symlink_metadata(&file_dst).unwrap().is_symlink());
    } else {
        assert!(!file_dst.exists());
    }
}

#[cfg(unix)]
#[test]
fn a_source_replaced_by_a_directory_leaves_no_partial_file_copy() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let destination = root.path().join("copy");
    let error = copy_file(
        &source,
        &destination,
        &fs::metadata(&source).unwrap(),
        Purpose::Copy,
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::IsADirectory);
    assert!(!destination.exists());
    assert!(source.is_dir());
}
