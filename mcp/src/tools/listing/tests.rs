use super::*;

#[test]
fn directory_information_identifies_the_directory_type() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let result = server
        .get_file_info(Parameters(PathParam { path: ".".into() }))
        .unwrap();
    assert_eq!(crate::tests::tool_json(&result)["type"], "directory");
    drop(server);

    let regular = root.path().join("plain");
    fs::write(&regular, b"hi").unwrap();
    let meta = fs::symlink_metadata(&regular).unwrap();
    let mut info = json!({});
    append_file_metadata(&mut info, &regular, &meta, None);
    assert!(info.get("modified_unix").is_none());
    append_symlink_target(&mut info, true, &regular);
    assert!(info.get("symlink_target").is_none());
}
