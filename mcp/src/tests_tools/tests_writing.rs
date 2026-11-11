//! Cross-platform unit tests for `writing` and `symlinks` tools.

use super::{server_at, tool_json, Parameters, TempDir};
use crate::tools::writing::{AppendFileParams, CreateFileParams, WriteFileParams};
use crate::tools::PathParam;
use serde_json::json;
use std::fs;

#[test]
fn create_file_builds_parent_dirs_defaults_empty_and_refuses_existing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);

    let nested = root.join("sub").join("fresh.txt");
    let created = server
        .create_file(Parameters(CreateFileParams {
            path: nested.display().to_string(),
            content: Some("initial".to_owned()),
        }))
        .expect("create nested file");
    assert_eq!(tool_json(&created)["size"], 7);

    let dup = server
        .create_file(Parameters(CreateFileParams {
            path: nested.display().to_string(),
            content: Some("clobber".to_owned()),
        }))
        .expect_err("must reject existing file");
    assert!(dup.message.contains("file already exists"));
    assert_eq!(fs::read_to_string(&nested).expect("read"), "initial");

    let empty = root.join("empty.txt");
    let res = server
        .create_file(Parameters(CreateFileParams {
            path: empty.display().to_string(),
            content: None,
        }))
        .expect("create empty file");
    assert_eq!(tool_json(&res)["size"], 0);
}

#[test]
fn write_file_creates_nested_file_and_truncates_on_overwrite() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let path = root.join("deep").join("doc.txt");

    let first = server
        .write_file(Parameters(WriteFileParams {
            path: path.display().to_string(),
            content: "long initial payload".to_owned(),
        }))
        .expect("write new file");
    assert_eq!(tool_json(&first)["size"], 20);

    let second = server
        .write_file(Parameters(WriteFileParams {
            path: path.display().to_string(),
            content: "ok".to_owned(),
        }))
        .expect("overwrite file");
    assert_eq!(tool_json(&second)["size"], 2);
    assert_eq!(fs::read_to_string(&path).expect("read"), "ok");
}

#[test]
fn append_file_creates_appends_and_rejects_directories() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let log = root.join("logs").join("events.log");

    server
        .append_file(Parameters(AppendFileParams {
            path: log.display().to_string(),
            content: "alpha".to_owned(),
        }))
        .expect("create via append");
    let res = server
        .append_file(Parameters(AppendFileParams {
            path: log.display().to_string(),
            content: "-beta".to_owned(),
        }))
        .expect("append second chunk");
    assert_eq!(tool_json(&res)["bytes_added"], 5);
    assert_eq!(fs::read_to_string(&log).expect("read"), "alpha-beta");

    let subdir = root.join("logs");
    let err = server
        .append_file(Parameters(AppendFileParams {
            path: subdir.display().to_string(),
            content: "bad".to_owned(),
        }))
        .expect_err("directory must be rejected");
    assert!(err.message.contains("regular file"));
}

#[test]
fn symlink_tools_reject_plain_files_and_roundtrip_links() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let target = root.join("target.txt");
    fs::write(&target, "body").expect("write target");

    let not_link = server.read_symlink(Parameters(PathParam {
        path: target.display().to_string(),
    }));
    assert!(not_link.is_err(), "plain file is not a symlink");

    let link = root.join("alias.txt");
    let args = json!({
        "link": link.display().to_string(),
        "target": target.display().to_string(),
    });
    match server.create_symlink(Parameters(serde_json::from_value(args).expect("params"))) {
        Ok(created) => {
            assert!(tool_json(&created)["target"].as_str().is_some());
            let read = server
                .read_symlink(Parameters(PathParam {
                    path: link.display().to_string(),
                }))
                .expect("read created symlink");
            assert_eq!(tool_json(&read)["target_exists"], true);
        }
        #[cfg(windows)]
        Err(err) if err.message.contains("1314") => {}
        Err(err) => panic!("unexpected create_symlink failure: {}", err.message),
    }
}
