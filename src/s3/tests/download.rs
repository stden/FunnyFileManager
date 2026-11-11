//! Purpose: unit tests for `S3Client::download_object`: payloads, staging files and failures.

use super::*;

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn get_event(bucket: &str, key: &str, body: &str) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri(format!("https://s3.us-east-1.amazonaws.com/{bucket}/{key}"))
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from(body.to_owned()))
            .expect("response"),
    )
}

#[test]
fn test_download_object_error_response() {
    let client = build_test_client(vec![error_event(403)]);
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("out.txt");
    let result = client.download_object("bucket", "key", &path);
    assert!(result.is_err());
}

#[test]
fn test_download_no_part_file_after_success() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("clean.txt");

    let client = build_test_client(vec![get_event("bucket", "clean.txt", "content")]);
    client.download_object("bucket", "clean.txt", &download_path)?;

    assert!(download_path.exists());
    // The staging file must be gone — check the whole directory, not just the
    // old `.part` name, so any leftover temp file fails the test.
    let leftovers: Vec<PathBuf> = fs::read_dir(temp_dir.path())?
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p != &download_path)
        .collect();
    assert_eq!(leftovers.len(), 0, "temp files left behind: {leftovers:?}");
    assert_eq!(fs::read_to_string(&download_path)?, "content");
    Ok(())
}

/// Regression: the staging path came from `with_extension("part")`, which
/// *replaces* the extension — downloading `report.txt` overwrote an unrelated
/// `report.part` sitting in the same directory and then renamed it away.
#[test]
fn test_download_leaves_unrelated_part_file_alone() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("report.txt");
    let bystander = temp_dir.path().join("report.part");
    fs::write(&bystander, "do not touch")?;

    let client = build_test_client(vec![get_event("bucket", "report.txt", "downloaded")]);
    client.download_object("bucket", "report.txt", &download_path)?;

    assert_eq!(fs::read_to_string(&download_path)?, "downloaded");
    assert_eq!(fs::read_to_string(&bystander)?, "do not touch");
    Ok(())
}

/// A destination that itself ends in `.part` downloads normally and leaves no
/// staging file behind.
#[test]
fn test_download_to_a_part_destination_leaves_no_staging_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("archive.part");

    let client = build_test_client(vec![get_event("bucket", "archive.part", "payload")]);
    client.download_object("bucket", "archive.part", &download_path)?;

    assert_eq!(fs::read_to_string(&download_path)?, "payload");
    let leftovers: Vec<PathBuf> = fs::read_dir(temp_dir.path())?
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path != &download_path)
        .collect();
    assert_eq!(leftovers.len(), 0, "temp files left behind: {leftovers:?}");
    Ok(())
}

#[test]
fn test_download_preserves_existing_file_on_failure() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("existing.txt");
    fs::write(&download_path, "original")?;

    let client = build_test_client(vec![error_event(500)]);
    let result = client.download_object("bucket", "existing.txt", &download_path);
    assert!(result.is_err());

    // Original file should be preserved
    assert_eq!(fs::read_to_string(&download_path)?, "original");
    Ok(())
}

#[test]
fn test_download_object_binary_data() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("binary.bin");
    let binary_data: Vec<u8> = (0..255).collect();

    let get_response = ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/bucket/binary.bin")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from(binary_data.clone()))
            .expect("response"),
    );
    let client = build_test_client(vec![get_response]);
    client.download_object("bucket", "binary.bin", &download_path)?;
    let written = fs::read(&download_path)?;
    assert_eq!(written, binary_data);
    Ok(())
}

#[test]
fn test_download_rejects_dangling_symlink() -> io::Result<()> {
    let dir = TempDir::new()?;
    let target = dir.path().join("missing-target.txt");
    let link = dir.path().join("link.txt");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(&target, &link).is_ok();
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file(&target, &link).is_ok();
    if !created {
        return Ok(());
    }
    let client = build_test_client(vec![get_event("bucket", "key", "payload")]);
    assert!(client.download_object("bucket", "key", &link).is_err());
    assert!(!target.exists());
    Ok(())
}

#[test]
fn test_download_rejects_oversized_content_length_without_overwriting() -> io::Result<()> {
    let dir = TempDir::new()?;
    let dest = dir.path().join("keep.txt");
    fs::write(&dest, "intact")?;

    let too_big = (100_u64 * 1024 * 1024 + 1).to_string();
    let event = ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/bucket/huge.bin")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .header("content-length", too_big)
            .body(SdkBody::from("tiny"))
            .expect("response"),
    );
    let client = build_test_client(vec![event]);
    let err = client
        .download_object("bucket", "huge.bin", &dest)
        .expect_err("must reject >100 MiB");
    assert!(err.to_string().contains("100 MiB"), "{err}");
    assert_eq!(fs::read_to_string(&dest)?, "intact");
    Ok(())
}

#[test]
fn test_download_rejects_oversized_body_when_content_length_header_is_absent() -> io::Result<()> {
    let dir = TempDir::new()?;
    let dest = dir.path().join("keep.txt");
    fs::write(&dest, "intact")?;

    let client = build_test_client(vec![get_event("bucket", "stream.bin", "12345")]);
    let err = client
        .download_object_limited("bucket", "stream.bin", &dest, 4)
        .expect_err("body exceeds configured limit");
    assert!(err.to_string().contains("100 MiB"), "{err}");
    assert_eq!(fs::read_to_string(&dest)?, "intact");
    Ok(())
}
