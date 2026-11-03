use super::*;
use crate::tests::tool_json;

#[cfg(unix)]
mod permissions;

fn server(root: &tempfile::TempDir) -> FfmServer {
    FfmServer::with_config(root.path().canonicalize().unwrap(), false, false)
}

#[test]
fn writing_and_appending_create_missing_parent_directories() {
    let root = tempfile::tempdir().unwrap();
    let server = server(&root);
    let result = server
        .write_file(Parameters(WriteFileParams {
            path: "nested/new".into(),
            content: "first".into(),
        }))
        .unwrap();
    assert_eq!(tool_json(&result)["size"], 5);
    assert_eq!(
        fs::read_to_string(root.path().join("nested/new")).unwrap(),
        "first"
    );
    let result = server
        .append_file(Parameters(AppendFileParams {
            path: "other/new".into(),
            content: "second".into(),
        }))
        .unwrap();
    drop(server);
    assert_eq!(tool_json(&result)["bytes_added"], 6);
    assert_eq!(
        fs::read_to_string(root.path().join("other/new")).unwrap(),
        "second"
    );
}

#[test]
fn append_refuses_directories_without_changing_their_contents() {
    let root = tempfile::tempdir().unwrap();
    let error = server(&root)
        .append_file(Parameters(AppendFileParams {
            path: root.path().display().to_string(),
            content: "bytes".into(),
        }))
        .unwrap_err();
    assert!(error.message.contains("regular file"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn a_write_failure_is_an_internal_error_and_preserves_the_original_bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("readonly-handle");
    fs::write(&path, b"original").unwrap();
    let mut file = fs::File::open(&path).unwrap();
    let error = write_content(&mut file, "replacement").unwrap_err();
    assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
    assert!(!error.message.is_empty());
    assert_eq!(fs::read(path).unwrap(), b"original");
}

#[test]
fn a_path_without_a_parent_needs_no_directory_creation() {
    create_parent(std::path::Path::new("")).unwrap();
}

#[test]
fn create_file_maps_non_collision_open_errors_to_internal_error() {
    let root = tempfile::tempdir().unwrap();
    let server = server(&root);
    let error = server
        .create_file(Parameters(CreateFileParams {
            path: "a".repeat(300),
            content: Some("data".into()),
        }))
        .unwrap_err();
    drop(server);
    assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
}
