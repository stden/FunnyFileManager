//! Additional cross-platform coverage for symlink copy, path confinement, replace, and cleanup.

use super::{make_dir_link_cross, server_at, tool_json, write_file_at, Parameters, TempDir};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::fs;
#[cfg(windows)]
use std::path::Path;

fn params<T: DeserializeOwned>(val: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(val).expect("params"))
}

#[test]
fn copy_entry_symlink_dir_and_path_resolution_reject_invalid_ancestors() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let target = root.join("real_target");
    let link = root.join("dir_link");
    write_file_at(&target.join("note.txt"), "hello");
    make_dir_link_cross(&target, &link);

    let copy_res = crate::fsops::copy_entry(
        &link,
        &root.join("copied_link"),
        crate::fsops::Purpose::Copy,
    );
    drop(copy_res);

    assert!(server.safe_path_nofollow_entry(".").is_ok());
    assert!(server.safe_path_create("absent_parent/..").is_err());
    #[cfg(windows)]
    if let Some(drive) = ('Q'..='Z').find(|d| !Path::new(&format!("{d}:\\")).exists()) {
        assert!(server
            .safe_path_create(&format!("{drive}:\\missing_dir\\leaf.txt"))
            .is_err());
    }
    drop(server);
}

#[test]
fn replace_and_delete_by_pattern_skip_links_and_non_utf8_files() {
    let (dir, outside) = (TempDir::new().expect("temp"), TempDir::new().expect("out"));
    let (server, root) = server_at(&dir);
    let keep = outside.path().join("keep.txt");
    write_file_at(&keep, "needle");
    make_dir_link_cross(outside.path(), &root.join("linked_dir"));
    fs::write(root.join("binary.txt"), [0xFF, 0xFE, b'n']).expect("binary");
    write_file_at(&root.join("valid.txt"), "needle");
    let root_s = root.display().to_string();

    let r_arg = json!({"path": root_s, "find": "needle", "replace": "ok"});
    server.replace(params(r_arg)).expect("replace");
    assert_eq!(
        fs::read_to_string(root.join("valid.txt")).expect("ok"),
        "ok"
    );
    assert_eq!(fs::read_to_string(&keep).expect("intact"), "needle");

    let d_arg = json!({"path": root_s, "pattern": "*.txt", "recursive": true});
    let del = server.delete_by_pattern(params(d_arg)).expect("delete");
    assert_eq!(tool_json(&del)["deleted_count"], 2);
    assert!(keep.is_file());
    drop(server);
}
