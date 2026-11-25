//! Cross-platform walker, content search, copy, and symlink confinement tests.

use super::{make_dir_link_cross, server_at, tool_json, write_file_at, Parameters, TempDir};
use crate::walk::{collect_files, content_kind};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn p<T: DeserializeOwned>(val: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(val).expect("params"))
}

fn try_link_file(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}

#[test]
fn walk_budget_missing_dir_and_content_kind_cover_all_categories() {
    let dir = TempDir::new().expect("temp");
    let root = dir.path();
    write_file_at(&root.join("a.exe"), "x");
    write_file_at(&root.join("b.json"), "y");
    let mut out = Vec::new();
    collect_files(root, root, &mut out, 0);
    assert!(out.is_empty());
    collect_files(root, &root.join("missing"), &mut out, 10);
    assert!(out.is_empty());
    collect_files(root, root, &mut out, 1);
    assert_eq!(out.len(), 1);
    assert_eq!(content_kind("app.exe"), "program");
    assert_eq!(content_kind("cfg.toml"), "data");
    assert_eq!(content_kind("blob.weird"), "other");
}

#[test]
fn grep_handles_unicode_empty_pattern_binary_and_budget() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("uni.txt"), "Привет мир\nsecond\n");
    fs::write(root.join("bin.txt"), [0xFF, 0xFE, b'\n']).unwrap();
    let root_s = root.display().to_string();
    let uni = server
        .grep(p(
            json!({"path": root_s, "pattern": "привет", "max_results": 1}),
        ))
        .unwrap();
    assert_eq!(tool_json(&uni)["count"], 1);
    let empty_pat = server
        .grep(p(
            json!({"path": root_s, "pattern": "", "extension": "txt", "max_results": 1}),
        ))
        .unwrap();
    drop(server);
    assert_eq!(tool_json(&empty_pat)["count"], 1);
}

#[test]
fn symlinks_create_read_list_sort_and_find_across_platforms() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let tf = root.join("real.txt");
    let td = root.join("real_dir");
    let fl = root.join("lf.txt");
    write_file_at(&tf, "hello");
    write_file_at(&td.join("child.txt"), "inside");
    if !try_link_file(&tf, &fl) {
        return;
    }
    fs::remove_file(&fl).unwrap();
    let fl_s = fl.display().to_string();
    server
        .create_symlink(p(json!({"link": fl_s, "target": tf.display().to_string()})))
        .unwrap();
    server.create_symlink(p(json!({"link": root.join("ld").display().to_string(), "target": td.display().to_string()}))).unwrap();
    let rd = server.read_symlink(p(json!({"path": fl_s}))).unwrap();
    assert_eq!(tool_json(&rd)["target_exists"], true);
    let root_s = root.display().to_string();
    server.list_directory(p(json!({"path": root_s}))).unwrap();
    server.get_file_info(p(json!({"path": fl_s}))).unwrap();
    server
        .sort_directory(p(json!({"path": root_s, "sort": "name"})))
        .unwrap();
    let found = server
        .find(p(json!({"path": root_s, "file_type": "symlink"})))
        .unwrap();
    drop(server);
    assert_eq!(tool_json(&found)["count"], 2);
}

#[test]
fn symlink_copy_dangling_rejection_and_copy_into_self_are_enforced() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let (f, d, lf) = (root.join("f.txt"), root.join("d"), root.join("lf.txt"));
    write_file_at(&f, "v");
    fs::create_dir(&d).unwrap();
    if !try_link_file(&f, &lf) {
        return;
    }
    server.create_symlink(p(json!({"link": root.join("ld").display().to_string(), "target": d.display().to_string()}))).unwrap();
    server.copy(p(json!({"source": lf.display().to_string(), "destination": root.join("lf2.txt").display().to_string()}))).unwrap();
    server.copy(p(json!({"source": root.join("ld").display().to_string(), "destination": root.join("ld2").display().to_string()}))).unwrap();
    server
        .delete(p(json!({"path": root.join("ld2").display().to_string()})))
        .unwrap();
    server
        .delete(p(
            json!({"path": root.join("lf2.txt").display().to_string()}),
        ))
        .unwrap();
    fs::remove_file(&f).unwrap();
    assert!(crate::paths::sanitize_path(&root, &lf.display().to_string()).is_err());
    let self_cp = server.copy(p(json!({"source": d.display().to_string(), "destination": d.join("sub").display().to_string()}))).is_err();
    drop(server);
    assert!(self_cp);
}

#[test]
fn copy_into_self_device_source_and_report_file_guards_reject_inputs() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let (f, d) = (root.join("f.txt"), root.join("d"));
    write_file_at(&f, "v");
    fs::create_dir(&d).unwrap();
    let self_cp = server
        .copy(p(json!({
            "source": d.display().to_string(),
            "destination": d.join("sub").display().to_string()
        })))
        .is_err();
    assert!(self_cp);
    assert!(server
        .find_duplicates(p(json!({"path": f.display().to_string()})))
        .is_err());
    assert!(server
        .analyse_directory(p(json!({"path": f.display().to_string()})))
        .is_err());
    drop(server);
    let dev = if cfg!(windows) {
        r"\\.\NUL"
    } else {
        "/dev/null"
    };
    assert!(crate::fsops::copy_entry(
        Path::new(dev),
        &root.join("out"),
        crate::fsops::Purpose::Copy
    )
    .is_err());
}

#[test]
fn directory_links_exercise_listing_reading_finding_deleting_and_dangling_guard() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let target = root.join("real_dir");
    let link = root.join("link_dir");
    write_file_at(&target.join("child.txt"), "inside");
    make_dir_link_cross(&target, &link);
    let (root_s, link_s) = (root.display().to_string(), link.display().to_string());
    let rd = server.read_symlink(p(json!({"path": link_s}))).unwrap();
    assert_eq!(tool_json(&rd)["target_exists"], true);
    server.list_directory(p(json!({"path": root_s}))).unwrap();
    server.get_file_info(p(json!({"path": link_s}))).unwrap();
    server
        .sort_directory(p(json!({"path": root_s, "sort": "name"})))
        .unwrap();
    let found = server
        .find(p(json!({"path": root_s, "file_type": "symlink"})))
        .unwrap();
    assert_eq!(tool_json(&found)["count"], 1);
    fs::remove_dir_all(&target).unwrap();
    assert!(crate::paths::sanitize_path(&root, &link_s).is_err());
    server.delete(p(json!({"path": link_s}))).unwrap();
    drop(server);
}
