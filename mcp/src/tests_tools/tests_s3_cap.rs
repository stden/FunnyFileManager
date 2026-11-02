//! Tests for S3 size caps, profile validation, read-only guards, and upload checks.

use crate::tools::s3::make_s3_client;
use crate::tools::s3_transfer::{check_download_size, S3CopyParams, MAX_S3_DOWNLOAD_BYTES};
use crate::FfmServer;
use serde_json::json;

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
fn s3_download_size_check_accepts_exact_limit_and_unknown_length() {
    check_download_size(Some(MAX_S3_DOWNLOAD_BYTES.cast_signed())).expect("exact limit is allowed");
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
        let params = S3CopyParams {
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

#[tokio::test]
async fn make_s3_client_and_list_buckets_reject_invalid_profile_names() {
    let dir = super::TempDir::new().expect("temp dir");
    let (server, _) = super::server_at(&dir);
    let oversized = "p".repeat(129);
    for bad in [
        "",
        "../default",
        "team/dev",
        "team\\dev",
        "bad\0",
        &oversized,
    ] {
        let err = make_s3_client(bad)
            .await
            .expect_err("malformed profile must fail");
        assert!(err.message.contains("invalid AWS profile name"));

        let list_err = server
            .s3_list_buckets(super::Parameters(
                serde_json::from_value(json!({"profile": bad})).expect("params"),
            ))
            .await
            .expect_err("list_buckets must fail on malformed profile");
        assert!(list_err.message.contains("invalid AWS profile name"));
    }
    drop(server);
}

#[tokio::test]
async fn readonly_server_blocks_every_mutating_s3_transfer_tool() {
    let dir = super::TempDir::new().expect("temp dir");
    let root = dir.path().canonicalize().expect("canonicalize");
    let server = FfmServer::with_config(root.clone(), true, false);
    let local = root.join("item.txt").display().to_string();

    let xfer = json!({"profile": "default", "bucket": "b", "key": "k", "local_path": local});
    let dl = server
        .s3_download(super::Parameters(
            serde_json::from_value(xfer.clone()).expect("p"),
        ))
        .await
        .expect_err("readonly download");
    assert!(dl.message.contains("read-only"));

    let ul = server
        .s3_upload(super::Parameters(serde_json::from_value(xfer).expect("p")))
        .await
        .expect_err("readonly upload");
    assert!(ul.message.contains("read-only"));

    let del = server
        .s3_delete(super::Parameters(
            serde_json::from_value(json!({"profile": "default", "bucket": "b", "key": "k"}))
                .expect("p"),
        ))
        .await
        .expect_err("readonly delete");
    drop(server);
    assert!(del.message.contains("read-only"));
}

#[tokio::test]
async fn s3_upload_rejects_oversized_sparse_local_file_before_network() {
    let dir = super::TempDir::new().expect("temp dir");
    let (server, root) = super::server_at(&dir);
    let huge = root.join("huge.bin");
    let file = std::fs::File::create(&huge).expect("create sparse file");
    file.set_len(65 * 1024 * 1024).expect("set 65 MiB length");
    drop(file);

    let args = json!({
        "profile": "default",
        "bucket": "bucket",
        "key": "huge.bin",
        "local_path": huge.display().to_string(),
    });
    let err = server
        .s3_upload(super::Parameters(
            serde_json::from_value(args).expect("params"),
        ))
        .await
        .expect_err("65 MiB exceeds 64 MiB S3 upload cap");
    drop(server);
    assert!(err.message.contains("local file too large"));
}
