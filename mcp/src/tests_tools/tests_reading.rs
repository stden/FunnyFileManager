//! Purpose: `head_file` and `read_lines` read only what they are asked for, up to
//! the read cap.

use super::*;
use crate::tools::reading::{HeadParams, LineRangeParams, MAX_READ_BYTES};

// ═══════════════════════════════════════════════════════════════════
// READING PARTS OF A FILE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn head_file_returns_the_first_lines() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("log.txt"), "one\ntwo\nthree\nfour\n");

    let result = server
        .head_file(Parameters(HeadParams {
            path: root.join("log.txt").display().to_string(),
            lines: Some(2),
        }))
        .expect("head_file");
    let json = tool_json(&result);
    assert_eq!(json["content"], "one\ntwo");
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["reached_end"], false);
}

/// Asking for more than there is returns all of it, and says so.
#[test]
fn head_file_asking_for_more_than_there_is_returns_all_of_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("short.txt"), "only\n");

    let json = tool_json(
        &server
            .head_file(Parameters(HeadParams {
                path: root.join("short.txt").display().to_string(),
                lines: Some(100),
            }))
            .expect("head_file"),
    );
    assert_eq!(json["content"], "only");
    assert_eq!(json["reached_end"], true);
}

#[test]
fn head_file_refuses_a_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    assert!(server
        .head_file(Parameters(HeadParams {
            path: root.display().to_string(),
            lines: None,
        }))
        .is_err());
}

#[test]
fn read_lines_returns_the_window_it_was_asked_for() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\nd\ne\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: Some(4),
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc\nd");
    assert_eq!(json["from"], 2);
    assert_eq!(json["lines_returned"], 3);
}

/// No `to` means "to the end", and the answer says it got there.
#[test]
fn read_lines_without_an_end_runs_to_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: None,
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc");
    assert_eq!(json["reached_end"], true);
}

/// A backwards range is refused rather than silently returning nothing.
#[test]
fn read_lines_refuses_a_backwards_range() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\n");

    assert!(server
        .read_lines(Parameters(LineRangeParams {
            path: root.join("f.txt").display().to_string(),
            from: Some(5),
            to: Some(2),
        }))
        .is_err());
}

/// Asking for exactly every line consumes the file, and the answer must say
/// so — a paging client told `false` here asks for a page past the end.
#[test]
fn head_file_count_equal_to_total_reports_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .head_file(Parameters(HeadParams {
                path: root.join("f.txt").display().to_string(),
                lines: Some(3),
            }))
            .expect("head_file"),
    );
    assert_eq!(json["lines_returned"], 3);
    assert_eq!(json["reached_end"], true);
}

/// The same boundary through `read_lines`: `to` on the file's last line.
#[test]
fn read_lines_to_the_last_line_reports_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: Some(3),
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc");
    assert_eq!(json["reached_end"], true);
}

/// A file with no newline inside the read cap used to be slurped whole; the
/// read now gives up with a clean error instead of buffering without bound.
#[test]
fn line_readers_refuse_past_the_read_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let blob = "x".repeat(usize::try_from(MAX_READ_BYTES).expect("the cap fits in usize") + 16);
    write_file_at(&root.join("blob.txt"), &blob);

    assert!(server
        .head_file(Parameters(HeadParams {
            path: root.join("blob.txt").display().to_string(),
            lines: Some(1),
        }))
        .is_err());
    assert!(server
        .read_lines(Parameters(LineRangeParams {
            path: root.join("blob.txt").display().to_string(),
            from: Some(1),
            to: None,
        }))
        .is_err());
}
