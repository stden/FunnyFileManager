//! Purpose: A caller-supplied file name is one component, never a path.

use super::*;
use crate::naming::{join_child, unique_destination};
use crate::tools::utilities::UniqueNameParams;

// ═══════════════════════════════════════════════════════════════
// A caller-supplied file name is one component, never a path
// ═══════════════════════════════════════════════════════════════

#[test]
fn unique_name_rejects_a_traversing_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            filename: "../../../../etc/passwd".to_owned(),
        }))
        // Answering at all leaks whether the file is there: an existing target
        // comes back as "passwd_1", a missing one as "passwd".
        .expect_err("a file name may not walk out of the directory it is for");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn unique_name_rejects_an_absolute_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            // `Path::join` discards the base outright for an absolute operand.
            filename: "/etc/passwd".to_owned(),
        }))
        .expect_err("an absolute name is not a name");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn join_child_accepts_only_one_ordinary_component() {
    let base = Path::new("/srv/root");

    assert_eq!(
        join_child(base, "report.txt").expect("a plain name is fine"),
        base.join("report.txt")
    );
    for hostile in ["../escape", "sub/dir.txt", "/etc/passwd", "..", ".", ""] {
        assert!(
            join_child(base, hostile).is_err(),
            "'{hostile}' must not be joinable onto a confined directory"
        );
    }
}

#[test]
fn unique_destination_treats_dangling_symlink_as_taken() {
    let dir = TempDir::new().expect("temp dir");
    let taken = dir.path().join("report.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing-target", &taken).expect("symlink");
    #[cfg(windows)]
    if std::os::windows::fs::symlink_file("missing-target", &taken).is_err() {
        fs::write(&taken, b"fallback").expect("write");
    }

    let unique = unique_destination(dir.path(), "report.txt").expect("unique name");
    assert_eq!(unique, dir.path().join("report_1.txt"));
}
