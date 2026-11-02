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

#[cfg(unix)]
#[test]
fn a_source_replaced_by_a_directory_leaves_no_partial_file_copy() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let destination = root.path().join("copy");
    let error = copy_file(&source, &destination, &fs::metadata(&source).unwrap(), Purpose::Copy).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::IsADirectory);
    assert!(!destination.exists());
    assert!(source.is_dir());
}
