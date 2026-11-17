//! Purpose: Sizes: `disk_usage` totals, the `view_file` size cap, and gigabyte
//! formatting.

use super::*;
use crate::tools::reading::ViewFileParams;
use crate::tools::utilities::FormatSizeParams;
use crate::tools::PathParam;

// ═══════════════════════════════════════════════════════════════
// disk_usage / view_file / format_size
// ═══════════════════════════════════════════════════════════════

#[test]
fn disk_usage_reports_a_missing_path_instead_of_zero() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .disk_usage(Parameters(PathParam {
            path: "ghost".to_owned(),
        }))
        .expect_err("a size of 0 would read as an empty directory");

    assert!(
        error.message.contains("not found"),
        "got: {}",
        error.message
    );
}

#[test]
fn disk_usage_of_a_single_file_is_its_own_size() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "abcde");

    let result = server
        .disk_usage(Parameters(PathParam {
            path: "doc.txt".to_owned(),
        }))
        .expect("disk usage runs");

    assert_eq!(tool_json(&result)["size"], 5);
}

#[test]
fn view_file_refuses_a_file_over_the_size_limit() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let big = root.join("big.bin");
    // Sparse: the file reports 11 MB without 11 MB ever being written.
    let handle = fs::File::create(&big).expect("create");
    handle.set_len(11 * 1024 * 1024).expect("grow sparsely");
    drop(handle);

    let error = server
        .view_file(Parameters(ViewFileParams {
            path: "big.bin".to_owned(),
            offset: None,
            limit: None,
        }))
        .expect_err("11 MB is over the 10 MB ceiling");

    assert!(
        error.message.contains("file too large"),
        "got: {}",
        error.message
    );
    assert!(
        error.message.contains("11.00 MB"),
        "the refusal should say how big the file actually is, got: {}",
        error.message
    );
}

#[test]
fn format_size_switches_to_gigabytes() {
    let result = FfmServer::format_size_tool(Parameters(FormatSizeParams {
        bytes: 3 * 1024 * 1024 * 1024,
    }));

    assert_eq!(tool_json(&result)["formatted"], "3.00 GB");
}
