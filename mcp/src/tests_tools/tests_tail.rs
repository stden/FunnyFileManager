//! Purpose: `tail_file` returns the last lines of any file.

use super::*;
use crate::tools::tail::TailParams;

// ─── tail ───────────────────────────────────────────────────────────────────

#[test]
fn tail_file_returns_the_last_lines() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let body = numbered_lines(100);
    write_file_at(&root.join("log.txt"), &body);

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("log.txt").display().to_string(),
                lines: Some(3),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 3);
    assert_eq!(json["content"], "line 98\nline 99\nline 100");
}

/// Asking for more lines than the file has returns the file, not an error.
#[test]
fn tail_file_asking_for_more_than_there_is_returns_all_of_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("short.txt"), "one\ntwo\n");

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("short.txt").display().to_string(),
                lines: Some(50),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["content"], "one\ntwo");
}

/// The point of seeking from the end: a file far larger than one block still
/// returns only what was asked for.
#[test]
fn tail_file_reads_the_end_of_a_large_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // Comfortably more than the 8 KiB block the tail reads backwards in.
    let body = numbered_lines(20_000);
    write_file_at(&root.join("big.log"), &body);

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("big.log").display().to_string(),
                lines: Some(2),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["content"], "line 19999\nline 20000");
}

#[test]
fn tail_file_handles_an_empty_file_and_a_zero_request() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("empty.txt"), "");

    let empty = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("empty.txt").display().to_string(),
                lines: None,
            }))
            .expect("tail"),
    );
    assert_eq!(empty["lines_returned"], 0);

    write_file_at(&root.join("some.txt"), "a\nb\n");
    let none_wanted = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("some.txt").display().to_string(),
                lines: Some(0),
            }))
            .expect("tail"),
    );
    assert_eq!(none_wanted["lines_returned"], 0);
}

/// A file with no newlines is still tailed: the last (only) line, not the
/// whole file slurped as a failed line-count.
#[test]
fn tail_file_of_a_file_with_no_newlines_returns_the_whole_line() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("blob.bin"), &"x".repeat(100));

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("blob.bin").display().to_string(),
                lines: Some(5),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 1);
    assert_eq!(json["content"].as_str().map(str::len), Some(100));
}

#[test]
fn tail_file_never_splits_a_multibyte_utf8_character_at_the_cap_boundary() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let mut raw = "🦀".as_bytes().to_vec();
    raw.extend(std::iter::repeat_n(b'a', 10 * 1024 * 1024 - 8));
    raw.extend_from_slice(b"\nlast\n");
    let path = root.join("utf8_cap.log");
    fs::write(&path, &raw).expect("write log");

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: path.display().to_string(),
                lines: Some(5),
            }))
            .expect("tail"),
    );
    let content = json["content"].as_str().expect("string content");
    assert_eq!(json["lines_returned"], 2);
    assert!(
        !content.contains('\u{FFFD}'),
        "split UTF-8 codepoint leaked"
    );
    assert!(content.starts_with('a') && content.ends_with("\nlast"));
}
