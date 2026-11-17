//! Purpose: Pins how `.` and `..` components, the filesystem root and missing
//! parents are resolved before a path is confined.

use super::root_of;
use crate::paths::{normalize_lexically, sanitize_path_create, sanitize_path_nofollow};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ─── canonicalize / invalid paths ───────────────────────────────

#[test]
fn nofollow_rejects_a_path_under_a_missing_directory() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // No-follow keeps the last component verbatim, so only the parent gets
    // canonicalized — and a parent that is not there has to be an error, not
    // an unchecked path handed on to the filesystem.
    let error = sanitize_path_nofollow(&root, "no_such_dir/file.txt")
        .expect_err("the parent cannot be resolved");
    assert!(error.contains("cannot resolve path"), "got: {error}");
    assert!(
        error.contains("no_such_dir"),
        "the error should name the path that failed, got: {error}"
    );
}

#[test]
fn rejects_a_missing_path_that_has_no_final_component() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // "missing/.." does not exist and ends in `..`, so there is no name to
    // create — accepting it would mean creating something at the parent.
    let error =
        sanitize_path_create(&root, "missing/..").expect_err("there is no file name to create");
    assert!(error.contains("invalid path"), "got: {error}");
}

// ─── sanitize_path_nofollow ─────────────────────────────────────

#[test]
fn nofollow_accepts_a_path_that_ends_in_a_parent_component() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::create_dir(root.join("sub")).expect("mkdir");

    // `sub/..` has no final component to keep verbatim, so the no-follow path
    // has to fall back to canonicalizing the whole thing.
    let resolved = sanitize_path_nofollow(&root, "sub/..").expect("resolves back to the root");
    assert_eq!(resolved, root);
}

#[test]
fn nofollow_rejects_the_filesystem_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let error =
        sanitize_path_nofollow(&root, "/").expect_err("'/' has no final component and is outside");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}

// ─── normalize_lexically ────────────────────────────────────────

#[test]
fn normalize_lexically_drops_current_dir_components() {
    assert_eq!(
        normalize_lexically(Path::new("./a/b")),
        PathBuf::from("a/b")
    );
}

#[test]
fn normalize_lexically_folds_parent_components() {
    assert_eq!(normalize_lexically(Path::new("a/../b")), PathBuf::from("b"));
    assert_eq!(
        normalize_lexically(Path::new("/a/b/../c")),
        PathBuf::from("/a/c")
    );
}

#[test]
fn normalize_lexically_collapses_repeated_and_trailing_slashes() {
    assert_eq!(
        normalize_lexically(Path::new("/a//b/")),
        PathBuf::from("/a/b")
    );
}

#[test]
fn normalize_lexically_cannot_pop_past_the_filesystem_root() {
    // More `..` than components: the result must stay at `/` so that `confine`
    // still sees an absolute path to reject rather than a relative fragment.
    assert_eq!(
        normalize_lexically(Path::new("/a/../../..")),
        PathBuf::from("/")
    );
}

#[test]
fn create_folds_parent_components_that_stay_inside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // Nothing on disk to canonicalize, so only the lexical fold can decide —
    // and it must accept a `..` that never leaves the root.
    let resolved = sanitize_path_create(&root, "new/deep/../file.txt")
        .expect("a '..' that stays inside the root is allowed");
    assert_eq!(resolved, root.join("new").join("file.txt"));
}
