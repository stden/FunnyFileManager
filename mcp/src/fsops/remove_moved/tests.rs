use super::*;

#[test]
fn an_unreadable_source_folder_is_reported_and_its_copy_is_preserved() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("file");
    fs::write(&source, b"source").unwrap();
    let mut first = None;
    remove_contents(&source, root.path(), &mut first);
    let kept = first.unwrap();
    assert_eq!(kept.path, source);
    assert_eq!(kept.reason.kind(), io::ErrorKind::NotADirectory);
    assert_eq!(fs::read(source).unwrap(), b"source");
}

#[test]
fn enumeration_errors_do_not_stop_removal_of_entries_already_copied() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let copy = root.path().join("copy");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&copy).unwrap();
    fs::write(source.join("file"), b"safe").unwrap();
    fs::write(copy.join("file"), b"safe").unwrap();
    let entry = fs::read_dir(&source).unwrap().next().unwrap();
    let entries = [Err(io::ErrorKind::PermissionDenied.into()), entry];
    let mut first = None;
    remove_entries(&source, &copy, entries, &mut first);
    let kept = first.unwrap();
    assert_eq!(kept.path, source);
    assert_eq!(kept.reason.kind(), io::ErrorKind::PermissionDenied);
    assert!(!source.join("file").exists());
    assert_eq!(fs::read(copy.join("file")).unwrap(), b"safe");
}
