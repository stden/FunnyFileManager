use super::*;

#[test]
fn directory_links_are_skipped_by_collect_files() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    crate::tests_tools::make_dir_link_cross(outside.path(), &root.path().join("linked"));
    fs::write(root.path().join("local.txt"), "ok").unwrap();

    let mut files = Vec::new();
    collect_files(root.path(), root.path(), &mut files, 10);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "local.txt");
}

#[cfg(unix)]
#[test]
fn sockets_are_never_collected_as_hashable_files() {
    let root = tempfile::tempdir().unwrap();
    let socket = std::os::unix::net::UnixListener::bind(root.path().join("socket")).unwrap();
    let mut files = Vec::new();
    collect_files(root.path(), root.path(), &mut files, 10);
    assert!(files.is_empty());
    drop(socket);
}

#[test]
fn collection_stops_before_the_first_file_after_its_budget_and_records_nested_metadata() {
    let root = tempfile::tempdir().unwrap();
    for name in ["first", "second"] {
        fs::write(root.path().join(name), "bytes").unwrap();
    }
    let mut files = Vec::new();
    collect_files(root.path(), &root.path().join("missing"), &mut files, 10);
    assert!(files.is_empty());
    collect_files(root.path(), root.path(), &mut files, 0);
    assert!(files.is_empty());
    collect_files(root.path(), root.path(), &mut files, 1);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, 5);
    assert!(files[0].modified.is_some());

    let nested = tempfile::tempdir().unwrap();
    fs::create_dir_all(nested.path().join("sub/leaf")).unwrap();
    fs::write(nested.path().join("sub/leaf/item.rs"), "fn f() {}").unwrap();
    let mut nested_files = Vec::new();
    collect_files(nested.path(), nested.path(), &mut nested_files, 10);
    assert_eq!(nested_files.len(), 1);
    assert_eq!(nested_files[0].relative, "sub/leaf/item.rs");
}

#[test]
fn every_report_category_is_selected_by_extension_case_insensitively() {
    for (name, expected) in [
        ("code.RS", "source"),
        ("readme.MD", "document"),
        ("readme.MARKDOWN", "document"),
        ("photo.PNG", "image"),
        ("audio.MP3", "audio"),
        ("movie.MKV", "video"),
        ("bundle.ZIP", "archive"),
        ("binary.EXE", "program"),
        ("values.JSON", "data"),
        ("README", "other"),
        ("file.unknown", "other"),
        ("archive.verylongextension", "other"),
    ] {
        assert_eq!(content_kind(name), expected);
    }
    assert_eq!(
        relative_slash_path(Path::new("root"), Path::new("outside/file")),
        "outside/file"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_directories_produce_no_collected_files() {
    use std::os::unix::fs::PermissionsExt;
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o000)).unwrap();
        let mut files = Vec::new();
        collect_files(root.path(), root.path(), &mut files, 10);
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(files.is_empty());
    });
}
