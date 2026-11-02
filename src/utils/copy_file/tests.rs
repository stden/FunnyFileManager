//! Purpose: Direct unit tests for sparse zero-run hole preservation during file copies.

use super::{copy_keeping_holes, looks_sparse};
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
    let _ = looks_sparse(&input.metadata().unwrap());
    let mut output = std::fs::File::create(&dst).unwrap();
    copy_keeping_holes(&input, &mut output).unwrap();
    output.flush().unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), payload);
}
