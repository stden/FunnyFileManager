use super::*;

#[test]
fn moving_the_server_root_and_deleting_a_missing_entry_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let error = server
        .move_rename(Parameters(CopyMoveParams {
            source: ".".into(),
            destination: "other".into(),
        }))
        .unwrap_err();
    assert_eq!(error.message, "cannot delete or move root directory");
    let error = server
        .delete(Parameters(DeleteParams {
            path: "absent".into(),
            recursive: false,
        }))
        .unwrap_err();
    drop(server);
    assert!(error.message.contains("not found"));
    assert!(fs::read_dir(root.path()).unwrap().next().is_none());
}

#[test]
fn a_parentless_destination_needs_no_directory_creation() {
    make_parent(Path::new("")).unwrap();
}

#[test]
fn deleting_the_server_root_is_refused_without_removing_it() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let error = server.delete(Parameters(DeleteParams { path: ".".into(), recursive: true })).unwrap_err();
    assert_eq!(error.message, "cannot delete or move root directory");
    assert!(root.path().is_dir());
    drop(server);
}
