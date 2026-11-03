//! Edge-case tests for replace, bookmarks, batch/transfer root guards, cleanup, permissions, text format, stats, kbdx, and automation errors.

use super::{server_at, tool_json, write_file_at, Parameters, TempDir};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn p<T: DeserializeOwned>(val: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(val).expect("params"))
}

fn set_locked_state(dir: &Path, file: &Path, lock: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file;
        let mode = if lock { 0o555 } else { 0o755 };
        fs::set_permissions(dir, fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        drop(crate::fsops::apply_readonly(file, lock));
    }
}

#[test]
fn replace_recurses_and_records_size_and_readonly_errors() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("sub/nested.txt"), "swap_me");
    write_file_at(&root.join("huge.txt"), &"x".repeat(10_240));
    let no_write = root.join("no_write");
    fs::create_dir_all(&no_write).unwrap();
    let locked = no_write.join("locked.txt");
    write_file_at(&locked, "swap_me");
    set_locked_state(&no_write, &locked, true);
    let req1 = json!({"path": root.display().to_string(), "find": "swap_me", "replace": "done"});
    let out1 = tool_json(&server.replace(p(req1)).unwrap());
    set_locked_state(&no_write, &locked, false);
    assert_eq!(out1["total_replacements"], 1);
    assert_eq!(out1["errors"], 1);
    let req2 =
        json!({"path": root.display().to_string(), "find": "x", "replace": "y".repeat(1100)});
    let out2 = tool_json(&server.replace(p(req2)).unwrap());
    drop(server);
    assert!(out2["error_details"][0]["error"]
        .as_str()
        .unwrap()
        .contains("10 MiB"));
}

#[test]
fn bookmarks_batch_and_transfer_enforce_name_and_root_guards() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let root_s = root.display().to_string();
    for bad in ["", "bad\n", &"a".repeat(257)] {
        assert!(server
            .set_bookmark(p(json!({"name": bad, "path": root_s})))
            .is_err());
    }
    let dst = root.join("empty_batch").display().to_string();
    server
        .batch_copy(p(json!({"paths": [], "destination": dst})))
        .unwrap();
    let dst2 = root.join("empty_move").display().to_string();
    server
        .batch_move(p(json!({"paths": [], "destination": dst2})))
        .unwrap();
    let b_del = server.batch_delete(p(json!({"paths": [root_s]}))).unwrap();
    assert_eq!(tool_json(&b_del)["total_errors"], 1);
    assert!(server.delete(p(json!({"path": root_s}))).is_err());
    let mv_err = server
        .move_rename(p(json!({"source": root_s, "destination": dst2})))
        .is_err();
    drop(server);
    assert!(mv_err);
}

#[test]
fn cleanup_and_permissions_cover_remaining_branches() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    fs::create_dir(root.join("clean_empty")).unwrap();
    write_file_at(&root.join("clean_busy/inside.txt"), "keep");
    let c_req = json!({"path": root.display().to_string(), "pattern": "clean_*", "recursive": false, "include_dirs": true, "dry_run": false});
    let clean = tool_json(&server.delete_by_pattern(p(c_req)).unwrap());
    assert_eq!(
        (clean["deleted_count"].as_u64(), clean["errors"].as_u64()),
        (Some(1), Some(1))
    );
    let f = root.join("perm.txt");
    write_file_at(&f, "");
    let ro = server
        .permissions(p(
            json!({"path": f.display().to_string(), "readonly": true}),
        ))
        .unwrap();
    assert_eq!(tool_json(&ro)["readonly"], true);
    server
        .permissions(p(
            json!({"path": f.display().to_string(), "readonly": false}),
        ))
        .unwrap();
    drop(server);
}

#[test]
fn text_format_covers_empty_utf16be_and_lone_cr() {
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let empty = root.join("empty.txt");
    write_file_at(&empty, "");
    let empty_fmt = server
        .text_format(p(json!({"path": empty.display().to_string()})))
        .unwrap();
    assert_eq!(tool_json(&empty_fmt)["line_endings"], "none");
    let be = root.join("be.txt");
    fs::write(&be, b"\xFE\xFFa\rb\r").unwrap();
    let be_fmt = tool_json(
        &server
            .text_format(p(json!({"path": be.display().to_string()})))
            .unwrap(),
    );
    drop(server);
    assert_eq!(
        (
            be_fmt["byte_order_mark"].as_str(),
            be_fmt["line_endings"].as_str()
        ),
        (Some("utf-16be"), Some("cr"))
    );
}

#[test]
fn file_stats_cap_and_keepass_tool_errors_reject_invalid_inputs() {
    let dir = TempDir::new().expect("temp");
    let root = dir.path().canonicalize().unwrap();
    let server = crate::FfmServer::with_config(root.clone(), false, true);
    let big = root.join("big.log");
    fs::File::create(&big)
        .unwrap()
        .set_len(100 * 1024 * 1024 + 1)
        .unwrap();
    assert!(server
        .file_stats(p(json!({"path": big.display().to_string()})))
        .is_err());
    let vault = root.join("bad.kdbx");
    fs::write(&vault, b"not-a-vault").unwrap();
    let sub = root.join("sub");
    fs::create_dir(&sub).unwrap();
    let k1 = server
        .kbdx(p(
            json!({"path": vault.display().to_string(), "keyfile": sub.display().to_string()}),
        ))
        .unwrap_err();
    assert!(k1.message.contains("not a file"));
    let bad_key = root.join("bad.key");
    fs::write(&bad_key, b"").unwrap();
    let bad = server
        .kbdx(p(
            json!({"path": vault.display().to_string(), "keyfile": bad_key.display().to_string()}),
        ))
        .is_err();
    drop(server);
    assert!(bad);
}

#[test]
fn automation_manager_debug_and_error_display_are_informative() {
    assert!(
        format!("{:?}", crate::automation::SessionManager::default()).contains("SessionManager")
    );
    assert!(crate::automation::Error::UnknownSession
        .to_string()
        .contains("unknown"));
    assert!(crate::automation::Error::SessionLimit
        .to_string()
        .contains("four"));
    assert!(crate::automation::Error::Timeout
        .to_string()
        .contains("timed out"));
    assert!(crate::automation::Error::Io(std::io::Error::other("x"))
        .to_string()
        .contains("transport"));
}
