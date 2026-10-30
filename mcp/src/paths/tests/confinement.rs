//! Purpose: Pins which paths `sanitize_path` and `sanitize_path_create` accept
//! as inside the root, and which escapes and malformed inputs they refuse.

use super::root_of;
use crate::paths::{sanitize_path, sanitize_path_create};
use std::fs;
use tempfile::TempDir;

#[test]
fn accepts_path_inside_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::write(root.join("inside.txt"), "hi").expect("write file");

    let resolved = sanitize_path(&root, &root.join("inside.txt").display().to_string())
        .expect("path inside the root must be accepted");
    assert_eq!(resolved, root.join("inside.txt"));
}

#[test]
fn accepts_the_root_itself() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let resolved = sanitize_path(&root, &root.display().to_string()).expect("root is in root");
    assert_eq!(resolved, root);
}

#[test]
fn relative_paths_resolve_against_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::write(root.join("rel.txt"), "hi").expect("write file");

    let resolved = sanitize_path(&root, "rel.txt").expect("relative path inside the root");
    assert_eq!(resolved, root.join("rel.txt"));
}

#[test]
fn rejects_absolute_path_outside_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("outside temp dir");
    let root = root_of(&dir);
    let outside_file = outside.path().join("passwd");
    fs::write(&outside_file, "not really a password file").expect("write outside fixture");

    let error = sanitize_path(&root, &outside_file.display().to_string())
        .expect_err("absolute escape must be rejected");
    assert!(
        error.contains("outside the allowed root"),
        "error should name the guard, got: {error}"
    );
    assert!(
        error.contains(&root.display().to_string()),
        "error should name the root so an operator can diagnose it, got: {error}"
    );
}

#[test]
fn rejects_parent_directory_escape() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    sanitize_path(&root, "../..").expect_err("'../..' must not escape the root");
    let outside = root.join("sub").join("..").join("..");
    sanitize_path(&root, &outside.display().to_string())
        .expect_err("traversal through an existing dir must not escape the root");
}

#[test]
fn rejects_parent_escape_for_new_files() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // Parent does not exist, so only lexical normalization can catch this one.
    sanitize_path_create(&root, "new/../../evil.txt")
        .expect_err("create must not escape the root either");
}

#[cfg(windows)]
#[test]
fn rejects_windows_verbatim_parent_escape_and_drive_relative_paths() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let verbatim_escape = format!(r"{}\missing\..\..\outside.txt", root.display());
    sanitize_path_create(&root, &verbatim_escape)
        .expect_err("verbatim path with '..' must not escape the root");

    sanitize_path_create(&root, "C:evil.txt")
        .expect_err("drive-relative path must not replace the verbatim root");
    sanitize_path_create(&root, r"\evil.txt")
        .expect_err("root-relative path without drive must be rejected");
}

#[test]
fn accepts_new_file_inside_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let resolved = sanitize_path_create(&root, "deep/nested/file.txt").expect("new file in root");
    assert_eq!(resolved, root.join("deep/nested/file.txt"));
}

#[test]
fn rejects_empty_and_null_paths_before_anything_else() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    assert!(sanitize_path(&root, "")
        .expect_err("empty")
        .contains("empty"));
    assert!(sanitize_path(&root, "a\0b")
        .expect_err("null byte")
        .contains("null byte"));
}

#[test]
fn rejects_missing_parent_for_read_operations() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let error = sanitize_path(&root, "no/such/dir/file.txt").expect_err("missing parent");
    assert!(error.contains("parent directory does not exist"), "{error}");
}

// ─── confine ────────────────────────────────────────────────────

#[test]
fn rejects_a_sibling_that_only_shares_the_root_name_prefix() {
    let base_dir = TempDir::new().expect("temp dir");
    let base = base_dir
        .path()
        .canonicalize()
        .expect("canonicalize temp dir");
    // The classic containment bug: a textual `starts_with` says `/srv/data` is
    // a prefix of `/srv/database`, and the whole sibling tree leaks out.
    let root = base.join("data");
    let sibling = base.join("database");
    fs::create_dir(&root).expect("mkdir root");
    fs::create_dir(&sibling).expect("mkdir sibling");
    let secret = sibling.join("secret.txt");
    fs::write(&secret, "secret").expect("write file");

    let error = sanitize_path(&root, &secret.display().to_string())
        .expect_err("a name-prefix sibling is not inside the root");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}
