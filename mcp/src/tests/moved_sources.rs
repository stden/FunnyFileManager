//! Purpose: A move across filesystems removes from its source only what the
//! copy holds, reports truthfully what stayed, keeps the times a rename would
//! have kept, and leaves no partial copy when the copy itself fails.

use super::*;
use crate::fsops::{FORCE_REMOVAL_REFUSAL, FORCE_RENAME_FAILURE};
use crate::tools::batch::BatchParams;
use crate::tools::transfer::CopyMoveParams;
use std::time::{Duration, SystemTime};

const TREE: [&str; 3] = ["g1.txt", "g2.txt", "rootsub/r.txt"];

/// `mixed/{g1.txt, g2.txt, rootsub/r.txt}` under the root, and an empty `out`.
fn mixed(dir: &TempDir) -> (FfmServer, PathBuf) {
    let (server, root) = server_at(dir);
    for name in TREE {
        write_file_at(&root.join("mixed").join(name), name);
    }
    fs::create_dir(root.join("out")).expect("mkdir");
    (server, root)
}

/// Run `transfer` as a move across filesystems (`EXDEV` on every rename).
fn across_filesystems<T>(transfer: impl FnOnce() -> T) -> T {
    FORCE_RENAME_FAILURE.set(Some(io::ErrorKind::CrossesDevices));
    let result = transfer();
    FORCE_RENAME_FAILURE.set(None);
    result
}

/// The same, with `r.txt` refusing to go — the way a guest cannot remove a
/// file of a root-owned subfolder, which root itself can never reproduce.
fn keeping_r_txt<T>(transfer: impl FnOnce() -> T) -> T {
    FORCE_REMOVAL_REFUSAL.set(Some("r.txt"));
    let result = across_filesystems(transfer);
    FORCE_REMOVAL_REFUSAL.set(None);
    result
}

fn assert_copy_whole_and_r_txt_stayed(root: &Path) {
    for name in TREE {
        let copy = root.join("out/mixed").join(name);
        assert_eq!(fs::read_to_string(copy).unwrap(), name, "the copy is whole");
    }
    assert!(
        root.join("mixed/rootsub/r.txt").is_file(),
        "what would not go"
    );
    assert!(!root.join("mixed/g1.txt").exists(), "the rest went");
    assert!(!root.join("mixed/g2.txt").exists(), "the rest went");
}

fn move_mixed(server: &FfmServer) -> Result<CallToolResult, McpError> {
    server.move_rename(Parameters(CopyMoveParams {
        source: "mixed".to_owned(),
        destination: "out/mixed".to_owned(),
    }))
}

#[test]
fn move_rename_across_filesystems_reports_what_stayed() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = mixed(&dir);

    let error = keeping_r_txt(|| move_mixed(&server)).expect_err("part of the source stayed");

    let stayed = "the copy is complete, but this stayed in the source";
    assert!(error.message.contains(stayed), "{}", error.message);
    assert!(error.message.contains("r.txt"), "{}", error.message);
    let data = error.data.expect("the report says where the whole copy is");
    assert_eq!(data["to"], root.join("out/mixed").display().to_string());
    assert!(
        data["left_behind"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("r.txt")),
        "{data}"
    );
    assert_copy_whole_and_r_txt_stayed(&root);
}

#[test]
fn batch_move_across_filesystems_counts_the_entry_and_what_stayed() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = mixed(&dir);

    let result = keeping_r_txt(|| {
        server.batch_move(Parameters(BatchParams {
            paths: vec!["mixed".to_owned()],
            destination: Some("out".to_owned()),
            recursive: true,
        }))
    })
    .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["moved"][0], "mixed", "moved whole, and ...");
    assert_eq!(payload["total_errors"], 1, "... part of the source stayed");
    let reported = payload["errors"][0]["error"].as_str().unwrap_or_default();
    assert!(reported.contains("r.txt"), "{payload}");
    assert!(
        reported.contains("the copy is complete, but this stayed in the source"),
        "{payload}"
    );
    assert_copy_whole_and_r_txt_stayed(&root);
}

/// A copy that fails leaves nothing under the final name: a partial copy
/// there is indistinguishable from a complete one, and the source stays.
#[cfg(unix)]
#[test]
fn a_failed_copy_across_filesystems_leaves_no_partial_copy() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = mixed(&dir);
    // A socket is the one special file a test can make without privileges.
    let socket = std::os::unix::net::UnixListener::bind(root.join("mixed/rootsub/sock"));
    socket.expect("bind a socket in the source");

    across_filesystems(|| move_mixed(&server)).expect_err("a socket cannot be copied");

    assert!(
        fs::symlink_metadata(root.join("out/mixed")).is_err(),
        "the partial copy is removed"
    );
    for name in TREE {
        assert_eq!(
            fs::read_to_string(root.join("mixed").join(name)).unwrap(),
            name
        );
    }
}

/// What a rename keeps, the copy that stands in for it keeps too: the
/// modification times of every file and folder, as `mv` does.
#[test]
fn a_move_across_filesystems_keeps_modification_times() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = mixed(&dir);
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    for path in ["mixed/g1.txt", "mixed/rootsub"] {
        let handle = fs::File::open(root.join(path)).expect("open");
        let times = fs::FileTimes::new().set_modified(old);
        handle
            .set_times(times)
            .expect("set an old modification time");
    }

    across_filesystems(|| move_mixed(&server)).expect("the move lands");

    for path in ["out/mixed/g1.txt", "out/mixed/rootsub"] {
        let modified = fs::metadata(root.join(path)).unwrap().modified().unwrap();
        assert_eq!(modified, old, "{path} keeps its modification time");
    }
}
