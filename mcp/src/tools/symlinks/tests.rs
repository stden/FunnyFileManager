use super::*;
use crate::tests::tool_json;

#[test]
fn create_symlink_dispatches_file_and_dir_targets_and_rejects_taken_names() {
    let root = tempfile::tempdir().unwrap();
    let canon = root.path().canonicalize().unwrap();
    fs::write(canon.join("file.txt"), b"x").unwrap();
    fs::create_dir(canon.join("sub")).unwrap();
    fs::write(canon.join("taken"), b"occupied").unwrap();
    let server = FfmServer::with_config(canon, false, false);

    assert!(server
        .create_symlink(Parameters(SymlinkParams {
            link: "taken".into(),
            target: "file.txt".into(),
        }))
        .is_err());
    assert!(server
        .create_symlink(Parameters(SymlinkParams {
            link: "taken".into(),
            target: "sub".into(),
        }))
        .is_err());
    drop(server);
}

#[test]
fn create_and_read_directory_symlink_reports_link_and_target() {
    let root = tempfile::tempdir().unwrap();
    let canon = root.path().canonicalize().unwrap();
    let target_dir = canon.join("real_dir");
    fs::create_dir(&target_dir).unwrap();
    let server = FfmServer::with_config(canon.clone(), false, false);

    #[cfg(windows)]
    JUNCTION_FALLBACK.set(true);
    let created = server.create_symlink(Parameters(SymlinkParams {
        link: "dir_link".into(),
        target: "real_dir".into(),
    }));
    #[cfg(windows)]
    JUNCTION_FALLBACK.set(false);

    let created = created.unwrap();
    let payload = tool_json(&created);
    assert_eq!(
        payload["link"],
        canon.join("dir_link").display().to_string()
    );
    assert_eq!(payload["target"], target_dir.display().to_string());

    let read = server
        .read_symlink(Parameters(PathParam {
            path: "dir_link".into(),
        }))
        .unwrap();
    assert_eq!(tool_json(&read)["target_exists"], true);
    drop(server);
}
