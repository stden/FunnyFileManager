//! Purpose: `largest_files` ranks files by size above a floor.

use super::*;
use crate::tools::analysis::LargestParams;

#[test]
fn largest_files_ranks_by_size_biggest_first() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("small.txt"), "x");
    write_file_at(&root.join("nested/big.txt"), &"y".repeat(500));
    write_file_at(&root.join("medium.txt"), &"z".repeat(50));

    let json = tool_json(
        &server
            .largest_files(Parameters(LargestParams {
                path: root.display().to_string(),
                limit: Some(2),
                min_size: None,
            }))
            .expect("largest_files"),
    );
    let files = json["files"].as_array().expect("a list of files");
    assert_eq!(files.len(), 2, "the limit is honoured");
    assert_eq!(files[0]["size"], 500);
    assert!(files[0]["path"].as_str().expect("path").contains("big.txt"));
    assert_eq!(files[1]["size"], 50);
    // The total covers everything considered, not just what was reported.
    assert_eq!(json["total_bytes"], 551);
    // A complete walk must say so — the same flag analyse_directory carries,
    // so a budget-stopped partial ranking is never silent.
    assert_eq!(json["truncated"], false);
}

#[test]
fn largest_files_skips_what_is_under_the_floor() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("tiny"), "x");
    write_file_at(&root.join("big"), &"y".repeat(100));

    let json = tool_json(
        &server
            .largest_files(Parameters(LargestParams {
                path: root.display().to_string(),
                limit: None,
                min_size: Some(10),
            }))
            .expect("largest_files"),
    );
    assert_eq!(json["files"].as_array().expect("files").len(), 1);
}

#[test]
fn largest_files_refuses_a_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "x");
    assert!(server
        .largest_files(Parameters(LargestParams {
            path: root.join("f.txt").display().to_string(),
            limit: None,
            min_size: None,
        }))
        .is_err());
}
