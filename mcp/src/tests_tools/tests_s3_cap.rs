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

#[test]
fn s3_copy_rejects_traversal_percent_query_and_control_characters() {
    let dir = super::TempDir::new().expect("temp dir");
    let (server, _) = super::server_at(&dir);
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("rt");
    for (src_bucket, src_key) in [
        ("", "key"),
        ("bucket", ""),
        ("bucket/../../other", "key"),
        ("bucket%2fother", "key"),
        ("bucket", "key%2e%2e"),
        ("bucket", "key?versionId=2"),
        ("bucket", "key#frag"),
        ("bucket", "key\n"),
    ] {
        let params = crate::tools::s3_transfer::S3CopyParams {
            profile: "default".to_owned(),
            source_bucket: src_bucket.to_owned(),
            source_key: src_key.to_owned(),
            dest_bucket: "dst".to_owned(),
            dest_key: "out".to_owned(),
        };
        let err = rt
            .block_on(server.s3_copy(super::Parameters(params)))
            .expect_err("bad");
        assert!(err.message.contains("invalid source"), "{}", err.message);
    }
}
