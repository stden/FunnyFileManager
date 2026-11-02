use super::*;

#[test]
fn duplicate_search_refuses_a_file_and_subdirectory_totals_include_nested_files() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), b"file").unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let params = serde_json::from_value(json!({"path":"file"})).unwrap();
    let error = find_duplicates(&server, &params).unwrap_err();
    assert!(error.message.contains("not a directory"));
    let files = [CollectedFile { path: root.path().join("child/file"), relative: "child/file".into(), name: "file".into(), size: 4 }];
    let children = child_directories(root.path(), &files);
    assert_eq!(children, vec![json!({"name":"child","size":4,"size_human":"4 B"})]);
    drop(server);
}
