//! Purpose: A move whose rename the filesystem refuses fails the way `mv`
//! does — the refusal reported, both sides untouched, no copy left behind.
//! Only a rename across filesystems may turn into a copy and a removal.

use super::*;
use crate::fsops::FORCE_RENAME_FAILURE;
use crate::tools::batch::BatchParams;
use crate::tools::transfer::CopyMoveParams;

/// `doc.txt` and `dir/inner.txt` under the root, and an empty `out`.
fn sources(dir: &TempDir) -> (FfmServer, PathBuf) {
    let (server, root) = server_at(dir);
    write_file_at(&root.join("doc.txt"), "note");
    write_file_at(&root.join("dir").join("inner.txt"), "inner");
    fs::create_dir(root.join("out")).expect("mkdir");
    (server, root)
}

/// Run `transfer` with every rename refused the way the kernel refuses a
/// user who may not write to the source's folder (`EACCES`, not `EXDEV`).
fn with_renames_denied<T>(transfer: impl FnOnce() -> T) -> T {
    FORCE_RENAME_FAILURE.set(Some(io::ErrorKind::PermissionDenied));
    let result = transfer();
    FORCE_RENAME_FAILURE.set(None);
    result
}

fn assert_untouched(root: &Path) {
    assert_eq!(fs::read_to_string(root.join("doc.txt")).unwrap(), "note");
    assert_eq!(
        fs::read_to_string(root.join("dir/inner.txt")).unwrap(),
        "inner"
    );
    let copies: Vec<_> = fs::read_dir(root.join("out")).unwrap().collect();
    assert_eq!(copies.len(), 0, "nothing is copied: {copies:?}");
}

fn move_one(server: &FfmServer, source: &str, destination: &str) -> Result<(), String> {
    server
        .move_rename(Parameters(CopyMoveParams {
            source: source.to_owned(),
            destination: destination.to_owned(),
        }))
        .map(|_| ())
        .map_err(|error| error.message.into_owned())
}

#[test]
fn a_refused_rename_of_a_file_copies_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = sources(&dir);

    let refusal = with_renames_denied(|| move_one(&server, "doc.txt", "out/doc.txt"))
        .expect_err("the refusal is the move's own failure");

    assert!(refusal.contains("forced rename failure"), "{refusal}");
    assert_untouched(&root);
}

#[test]
fn a_refused_rename_of_a_folder_copies_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = sources(&dir);

    let refusal = with_renames_denied(|| move_one(&server, "dir", "out/dir"))
        .expect_err("the refusal is the move's own failure");

    assert!(refusal.contains("forced rename failure"), "{refusal}");
    assert_untouched(&root);
}

#[test]
fn a_refused_rename_in_a_batch_copies_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = sources(&dir);

    let result = with_renames_denied(|| {
        server.batch_move(Parameters(BatchParams {
            paths: vec!["doc.txt".to_owned(), "dir".to_owned()],
            destination: Some("out".to_owned()),
            recursive: true,
        }))
    })
    .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["total_moved"], 0);
    assert_eq!(payload["total_errors"], 2);
    assert!(
        payload["errors"][0]["error"]
            .as_str()
            .unwrap_or_default()
            .contains("forced rename failure"),
        "{payload}"
    );
    assert_untouched(&root);
}

/// The same with a real refusal, no seam: the kernel will not rename a
/// folder over one that holds something (`ENOTEMPTY`). The copy fallback
/// used to merge the source into it and then remove the source.
#[test]
fn a_folder_is_never_merged_into_a_populated_folder() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = sources(&dir);
    write_file_at(&root.join("out").join("dir").join("theirs.txt"), "theirs");

    move_one(&server, "dir", "out/dir").expect_err("nothing is merged");

    assert_eq!(
        fs::read_to_string(root.join("dir/inner.txt")).unwrap(),
        "inner",
        "the source stays"
    );
    assert!(
        !root.join("out/dir/inner.txt").exists(),
        "nothing lands in the folder that was there"
    );
    assert_eq!(
        fs::read_to_string(root.join("out/dir/theirs.txt")).unwrap(),
        "theirs"
    );
}
