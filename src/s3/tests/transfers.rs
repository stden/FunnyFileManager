//! Purpose: unit tests for `S3Client` upload, copy and delete, alone and as one round trip.

use super::*;

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_s3_client_object_operations() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("download.txt");
    let upload_path = temp_dir.path().join("upload.txt");
    fs::write(&upload_path, "payload")?;

    let get_response = ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/alpha/download.txt")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from("downloaded"))
            .expect("response"),
    );
    let client = build_test_client(vec![get_response, ok_event(), ok_event(), ok_event()]);

    client.download_object("alpha", "download.txt", &download_path)?;
    assert_eq!(fs::read_to_string(&download_path)?, "downloaded");

    client.upload_object("alpha", "upload.txt", &upload_path)?;
    client.copy_object("alpha", "upload.txt", "alpha", "copied.txt")?;
    client.delete_object("alpha", "copied.txt")?;

    Ok(())
}

#[test]
fn test_upload_object_error_response() {
    let temp_dir = TempDir::new().expect("temp dir");
    let upload_path = temp_dir.path().join("file.txt");
    fs::write(&upload_path, "data").unwrap();
    let client = build_test_client(vec![error_event(500)]);
    let result = client.upload_object("bucket", "key", &upload_path);
    assert!(result.is_err());
}

#[test]
fn test_upload_nonexistent_file() {
    let client = build_test_client(vec![ok_event()]);
    let bad_path = PathBuf::from("/nonexistent/path/file.txt");
    let result = client.upload_object("bucket", "key", &bad_path);
    assert!(result.is_err());
}

#[test]
fn test_delete_object_error_response() {
    let client = build_test_client(vec![error_event(500)]);
    let result = client.delete_object("bucket", "key");
    assert!(result.is_err());
}

#[test]
fn test_copy_object_error_response() {
    let client = build_test_client(vec![error_event(403)]);
    let result = client.copy_object("src-bucket", "src-key", "dst-bucket", "dst-key");
    assert!(result.is_err());
}

#[test]
fn test_copy_object_cross_bucket() -> io::Result<()> {
    // Verify copy works across different buckets
    let client = build_test_client(vec![ok_event()]);
    client.copy_object(
        "source-bucket",
        "path/to/file.txt",
        "dest-bucket",
        "new/location.txt",
    )?;
    Ok(())
}
