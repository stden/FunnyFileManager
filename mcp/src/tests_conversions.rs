//! Boundary tests for the checked numeric conversions behind the tool reports:
//! human-readable sizes, client-supplied line counts, and S3 object sizes.

use super::*;
use crate::conversions::{object_size, saturating_usize};
use crate::tools::reading::ViewFileParams;
use rmcp::handler::server::wrapper::Parameters;
use std::fs;
use tempfile::TempDir;

#[test]
fn format_size_keeps_whole_bytes_below_one_kib() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn format_size_switches_unit_at_each_boundary() {
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1024 * 1024 - 1), "1024.00 KB");
    assert_eq!(format_size(1024 * 1024), "1.00 MB");
    assert_eq!(format_size(1024 * 1024 * 1024 - 1), "1024.00 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
}

/// The integer maths must round exactly as `{:.2}` on an `f64` did.
#[test]
fn format_size_rounds_to_the_nearest_hundredth_and_halves_to_even() {
    // 1029 B = 1.00488 KB and 1030 B = 1.00586 KB: either side of a half.
    assert_eq!(format_size(1029), "1.00 KB");
    assert_eq!(format_size(1030), "1.01 KB");
    // 1152 B = 1.125 KB and 1408 B = 1.375 KB: exact halves go to even.
    assert_eq!(format_size(1152), "1.12 KB");
    assert_eq!(format_size(1408), "1.38 KB");
}

#[test]
fn format_size_is_exact_up_to_the_largest_u64() {
    assert_eq!(format_size(u64::MAX), "17179869184.00 GB");
    // Past 2^53 an `f64` quotient loses the low bits and reports `.72` here.
    assert_eq!(format_size(4_868_450_995_711_151_744), "4534098315.71 GB");
}

#[test]
fn saturating_usize_keeps_every_value_that_fits() {
    assert_eq!(saturating_usize(0), 0);
    let largest = u64::try_from(usize::MAX).expect("usize is at most 64 bits here");
    assert_eq!(saturating_usize(largest), usize::MAX);
}

#[test]
fn saturating_usize_clamps_what_does_not_fit() {
    assert_eq!(saturating_usize(u64::MAX), usize::MAX);
    // Only reachable where `usize` is narrower than `u64`.
    if let Some(past) = u64::try_from(usize::MAX)
        .ok()
        .and_then(|max| max.checked_add(1))
    {
        assert_eq!(saturating_usize(past), usize::MAX);
    }
}

#[test]
fn object_size_passes_through_non_negative_sizes() {
    assert_eq!(object_size(Some(0)), 0);
    assert_eq!(object_size(Some(1)), 1);
    assert_eq!(object_size(Some(i64::MAX)), 9_223_372_036_854_775_807);
}

#[test]
fn object_size_reports_a_missing_or_negative_size_as_zero() {
    assert_eq!(object_size(None), 0);
    assert_eq!(object_size(Some(-1)), 0);
    assert_eq!(object_size(Some(i64::MIN)), 0);
}

/// A line window past every line is empty, not a wrap-around to the start.
#[test]
fn view_file_answers_an_offset_past_the_end_with_no_lines() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    fs::write(root.join("notes.txt"), "one\ntwo\n").expect("write fixture");
    let server = FfmServer::new(root);

    let result = server
        .view_file(Parameters(ViewFileParams {
            path: "notes.txt".to_owned(),
            offset: Some(u64::MAX),
            limit: Some(u64::MAX),
        }))
        .expect("an out-of-range window is not an error");
    drop(server);
    let text = result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a tool result carries one text content");
    let payload: serde_json::Value = serde_json::from_str(&text.text).expect("JSON payload");

    assert_eq!(payload["total_lines"], 2);
    assert_eq!(payload["lines_returned"], 0);
    assert_eq!(payload["content"], "");
}
