use super::*;

#[test]
fn duplicate_search_refuses_a_file_and_subdirectory_totals_include_nested_files() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), b"file").unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let params = serde_json::from_value(json!({"path":"file"})).unwrap();
    let error = find_duplicates(&server, &params).unwrap_err();
    assert!(error.message.contains("not a directory"));
    let files = [CollectedFile {
        path: root.path().join("child/file"),
        relative: "child/file".into(),
        name: "file".into(),
        size: 4,
        modified: None,
    }];
    let children = child_directories(root.path(), &files);
    assert_eq!(
        children,
        vec![json!({"name":"child","size":4,"size_human":"4 B"})]
    );
    assert_eq!(
        child_directories(&root.path().join("missing"), &[]),
        Vec::<serde_json::Value>::new()
    );
    drop(server);
}

#[test]
fn large_files_pre_filter_by_edges_and_verify_middle_bytes_with_sha256() {
    let root = tempfile::tempdir().unwrap();
    let mut base = vec![b'A'; 24 * 1024];
    base[12 * 1024] = b'M';
    let mut middle_diff = base.clone();
    middle_diff[12 * 1024] = b'X';
    let mut edge_diff = base.clone();
    edge_diff[0] = b'Z';

    fs::write(root.path().join("dup_a.bin"), &base).unwrap();
    fs::write(root.path().join("dup_b.bin"), &base).unwrap();
    fs::write(root.path().join("same_edges_diff_mid.bin"), &middle_diff).unwrap();
    fs::write(root.path().join("diff_edge.bin"), &edge_diff).unwrap();

    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let params = serde_json::from_value(json!({"path": "."})).unwrap();
    let report = crate::tests::tool_json(&find_duplicates(&server, &params).unwrap());
    assert_eq!(report["duplicate_groups"], 1);
    assert_eq!(report["groups"][0]["copies"], 2);
    assert_eq!(report["reclaimable"], 24 * 1024);
    drop(server);
}
