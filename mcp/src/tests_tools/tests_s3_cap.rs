//! Tests for the S3 download size cap (VULN-06).

use crate::tools::s3_transfer::{check_download_size, MAX_S3_DOWNLOAD_BYTES};

#[test]
fn s3_download_size_check_rejects_oversized_length() {
    let err = check_download_size(Some(MAX_S3_DOWNLOAD_BYTES.cast_signed() + 1))
        .expect_err("object exceeds the limit");
    assert!(
        err.message.contains("too large"),
        "error message must mention 'too large': {}",
        err.message
    );
}

#[test]
fn s3_download_size_check_accepts_exact_limit() {
    check_download_size(Some(MAX_S3_DOWNLOAD_BYTES.cast_signed())).expect("exact limit is allowed");
}

#[test]
fn s3_download_size_check_accepts_unknown_length() {
    check_download_size(None).expect("unknown content-length is allowed");
}
