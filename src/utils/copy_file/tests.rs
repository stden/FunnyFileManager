//! Purpose: Direct unit tests for sparse zero-run hole preservation during file copies.

use super::copy_keeping_holes;
use std::io::Write;

#[test]
fn copy_keeping_holes_preserves_zero_runs_and_data_blocks() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("sparse.bin");
    let dst = temp.path().join("out.bin");
    let mut payload = vec![0_u8; 64 * 1024];
    payload.extend_from_slice(b"hello-sparse");
    payload.extend(std::iter::repeat_n(0_u8, 64 * 1024));
    std::fs::write(&src, &payload).unwrap();
    let input = std::fs::File::open(&src).unwrap();
    let mut output = std::fs::File::create(&dst).unwrap();
    copy_keeping_holes(&input, &mut output).unwrap();
    output.flush().unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), payload);
}

#[cfg(unix)]
#[test]
fn copy_file_routes_truly_sparse_unix_files_through_hole_preserving_copy() {
    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("hole.bin");
    let dst = temp.path().join("hole_copy.bin");
    let created = std::fs::File::create(&src).unwrap();
    created.set_len(1024 * 1024).unwrap();
    drop(created);
    assert!(super::looks_sparse(&std::fs::metadata(&src).unwrap()));
    super::copy_file(&src, &dst, crate::utils::CopyRules::PLAIN).unwrap();
    assert_eq!(std::fs::metadata(&dst).unwrap().len(), 1024 * 1024);
}

#[test]
fn copy_file_removes_created_destination_when_preserving_times_fails() {
    use crate::utils::{CopyRules, Times, FORCE_TIMES_FAILURE};

    let temp = tempfile::TempDir::new().unwrap();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");
    std::fs::write(&src, b"payload").unwrap();
    let rules = CopyRules {
        times: Times::Source,
        ..CopyRules::PLAIN
    };

    FORCE_TIMES_FAILURE.set(true);
    let result = super::copy_file(&src, &dst, rules);
    FORCE_TIMES_FAILURE.set(false);

    assert!(result.is_err());
    assert!(!dst.exists(), "half-copied destination must be removed");
}

#[test]
fn open_regular_file_with_metadata_reports_regular_length_and_refuses_dirs() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("sample.bin");
    std::fs::write(&path, b"abcdef").unwrap();
    let (_file, meta) = super::open_regular_file_with_metadata(&path).unwrap();
    assert_eq!(meta.len(), 6);
    assert!(super::open_regular_file_with_metadata(temp.path()).is_err());
}
