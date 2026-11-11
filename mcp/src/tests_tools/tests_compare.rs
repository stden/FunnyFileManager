//! Purpose: Comparing two directories by name and size, and two files byte for
//! byte.

use super::*;
use crate::tools::comparing::{CompareFilesParams, CompareParams};

// ─── compare ────────────────────────────────────────────────────────────────

#[test]
fn compare_directories_names_what_differs() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("left/same.txt"), "body");
    write_file_at(&root.join("left/only-left.txt"), "x");
    write_file_at(&root.join("left/nested/grown.txt"), "short");
    write_file_at(&root.join("right/same.txt"), "body");
    write_file_at(&root.join("right/only-right.txt"), "y");
    write_file_at(&root.join("right/nested/grown.txt"), "much longer body");

    let json = tool_json(
        &server
            .compare_directories(Parameters(CompareParams {
                left: root.join("left").display().to_string(),
                right: root.join("right").display().to_string(),
            }))
            .expect("compare"),
    );

    assert_eq!(json["identical"], false);
    assert_eq!(json["only_in_left"], serde_json::json!(["only-left.txt"]));
    assert_eq!(json["only_in_right"], serde_json::json!(["only-right.txt"]));

    // The comparison reaches into subdirectories, not just the top level.
    let differing = json["differing_size"].as_array().expect("differing_size");
    assert_eq!(differing.len(), 1);
    assert!(differing[0]["path"]
        .as_str()
        .unwrap_or_default()
        .ends_with("grown.txt"));
}

#[test]
fn compare_directories_says_so_when_they_match() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    for side in ["left", "right"] {
        write_file_at(&root.join(side).join("a.txt"), "same");
        write_file_at(&root.join(side).join("deep/b.txt"), "same too");
    }

    let json = tool_json(
        &server
            .compare_directories(Parameters(CompareParams {
                left: root.join("left").display().to_string(),
                right: root.join("right").display().to_string(),
            }))
            .expect("compare"),
    );
    assert_eq!(json["identical"], true, "{json}");
}

#[test]
fn compare_files_calls_identical_files_identical() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "same content\n");
    write_file_at(&root.join("b"), "same content\n");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], true);
    assert!(json["sha256"].is_string());
    assert!(json["first_difference_at"].is_null());
}

/// Where they stop agreeing is the useful half of the answer.
#[test]
fn compare_files_reports_the_first_differing_offset() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "abcdef");
    write_file_at(&root.join("b"), "abcXef");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 3);
    assert_ne!(json["left_sha256"], json["right_sha256"]);
}

/// Different sizes settle "identical" without hashing — but the offset is
/// still read from the bytes: these two disagree at byte 0, not at byte 5.
#[test]
fn compare_files_answers_different_sizes_without_hashing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "short");
    write_file_at(&root.join("b"), "much longer content");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 0);
    assert!(json["left_sha256"].is_null(), "no hashing was needed");
}

/// When one file is a prefix of the other, the first difference is at the
/// shorter file's length — the first offset where only one still has a byte.
#[test]
fn compare_files_prefix_difference_is_at_the_shorter_length() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "abc");
    write_file_at(&root.join("b"), "abcdef");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 3);
}
