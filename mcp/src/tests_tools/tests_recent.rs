//! Purpose: `recent_files` lists what changed within its window, up to its limit.

use super::*;
use crate::tools::analysis::RecentParams;

// ─── recent ─────────────────────────────────────────────────────────────────

#[test]
fn recent_files_lists_what_was_just_written() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");
    write_file_at(&root.join("nested/also-fresh.txt"), "also now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(1),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 2, "{json}");
}

/// A window of zero days ends now, so nothing written a moment ago is inside
/// it — the cutoff is real rather than decorative.
#[test]
fn recent_files_respects_the_window() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(0),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 0, "{json}");
}

#[test]
fn recent_files_honours_the_limit() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    for index in 0..10 {
        write_file_at(&root.join(format!("file-{index}.txt")), "body");
    }

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(1),
                limit: Some(3),
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 3);
}

/// `days` is a JSON integer; a caller can send `u64::MAX`. Multiplying that
/// by seconds-per-day must not panic the server.
#[test]
fn recent_files_accepts_an_enormous_window_without_panicking() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(u64::MAX),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 1, "{json}");
}
