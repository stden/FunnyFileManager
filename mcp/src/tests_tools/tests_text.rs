//! Purpose: `text_format` reports a file's punctuation; `file_stats` counts it.

use super::*;
use crate::tools::text_format::MAX_FORMAT_BYTES;
use crate::tools::PathParam;

#[test]
fn text_format_names_the_line_endings_and_the_final_newline() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("unix.txt"), "a\nb\n");
    write_file_at(&root.join("dos.txt"), "a\r\nb\r\n");
    write_file_at(&root.join("mixed.txt"), "a\r\nb\nc");

    let of = |name: &str| {
        tool_json(
            &server
                .text_format(Parameters(PathParam {
                    path: root.join(name).display().to_string(),
                }))
                .expect("text_format"),
        )
    };

    assert_eq!(of("unix.txt")["line_endings"], "lf");
    assert_eq!(of("unix.txt")["final_newline"], true);
    assert_eq!(of("dos.txt")["line_endings"], "crlf");
    assert_eq!(of("dos.txt")["crlf_count"], 2);
    // A CRLF must not be counted as a CR and an LF as well.
    assert_eq!(of("dos.txt")["lf_count"], 0);
    assert_eq!(of("dos.txt")["cr_count"], 0);
    assert_eq!(of("mixed.txt")["line_endings"], "mixed");
    assert_eq!(of("mixed.txt")["final_newline"], false);
}

#[test]
fn text_format_reports_a_bom_and_a_binary_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    fs::write(root.join("bom.txt"), b"\xEF\xBB\xBFhello\n").expect("write");
    fs::write(root.join("bin.dat"), b"\x00\x01\x02").expect("write");

    let of = |name: &str| {
        tool_json(
            &server
                .text_format(Parameters(PathParam {
                    path: root.join(name).display().to_string(),
                }))
                .expect("text_format"),
        )
    };
    assert_eq!(of("bom.txt")["byte_order_mark"], "utf-8");
    assert_eq!(of("bom.txt")["binary"], false);
    assert_eq!(of("bin.dat")["binary"], true);
    assert_eq!(of("bin.dat")["line_endings"], "none");
}

/// The endings sample stops at a cap, and the verdicts must not be artifacts
/// of it: a CRLF pair cut at the cap is not a lone CR, and the final newline
/// lives at the end of the file, past the sample.
#[test]
fn text_format_is_not_fooled_by_the_sample_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // One odd leading byte lands a CRLF pair astride the sample boundary.
    let mut content = String::from("x");
    content.push_str(&"\r\n".repeat(MAX_FORMAT_BYTES / 2 + 1_000));
    write_file_at(&root.join("big.txt"), &content);

    let json = tool_json(
        &server
            .text_format(Parameters(PathParam {
                path: root.join("big.txt").display().to_string(),
            }))
            .expect("text_format"),
    );
    assert_eq!(json["line_endings"], "crlf");
    assert_eq!(json["cr_count"], 0, "the cut pair must not count as a CR");
    assert_eq!(json["final_newline"], true);
}

/// The mirror case: a CR that lands on the cap boundary but is NOT followed
/// by LF is a genuine lone CR, and dropping it would hide a real "mixed"
/// verdict inside the very sample the tool claims to judge from.
#[test]
fn text_format_keeps_a_genuine_lone_cr_at_the_sample_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // LF lines plus one filler byte put a lone CR exactly on the sample's
    // last byte, and the byte just past the cap is an ordinary letter, not LF.
    let mut content = "a\n".repeat(MAX_FORMAT_BYTES / 2 - 1);
    content.push('x');
    content.push('\r');
    content.push_str(&"b\n".repeat(2_000));
    write_file_at(&root.join("big.txt"), &content);

    let json = tool_json(
        &server
            .text_format(Parameters(PathParam {
                path: root.join("big.txt").display().to_string(),
            }))
            .expect("text_format"),
    );
    assert_eq!(json["line_endings"], "mixed");
    assert_eq!(json["cr_count"], 1, "a real lone CR at the cap must count");
    assert_eq!(json["final_newline"], true);
}

#[test]
fn file_stats_counts_lines_words_and_the_longest_line() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "one two\nthree four five\n");

    let json = tool_json(
        &server
            .file_stats(Parameters(PathParam {
                path: root.join("f.txt").display().to_string(),
            }))
            .expect("file_stats"),
    );
    assert_eq!(json["lines"], 2);
    assert_eq!(json["words"], 5);
    assert_eq!(json["bytes"], 24);
    assert_eq!(json["longest_line_number"], 2);
    assert_eq!(json["longest_line_chars"], 15);
}
