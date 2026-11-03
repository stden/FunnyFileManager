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
