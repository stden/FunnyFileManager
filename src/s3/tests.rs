//! Unit tests and shared test helpers for `s3`.
#![allow(unused_imports)]

use super::*;

use aws_sdk_s3::config::{Credentials, Region, SharedCredentialsProvider};

use aws_sdk_s3::Client;

use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};

use aws_smithy_types::body::SdkBody;

use http_1x::{Request, Response};

use std::env;

use std::fs;

use std::io;

use std::path::PathBuf;

use std::sync::MutexGuard;

use tempfile::TempDir;

pub fn build_test_client(events: Vec<ReplayEvent>) -> S3Client {
    let http_client = StaticReplayClient::new(events);
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .behavior_version_latest()
        .build();
    let client = Client::from_conf(config);
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    S3Client {
        client,
        runtime,
        profile: "test".to_string(),
        current_bucket: None,
        current_prefix: String::new(),
    }
}

/// Sandboxes `$HOME` / `%USERPROFILE%` under the crate-wide env lock so S3
/// tests cannot race command/history/editor tests that also mutate the process
/// environment.
pub struct AwsHomeEnvGuard {
    _lock: MutexGuard<'static, ()>,
    original_home: Option<String>,
    original_userprofile: Option<String>,
}

impl AwsHomeEnvGuard {
    pub(crate) fn set(path: &std::path::Path) -> Self {
        let lock = crate::test_support::env_lock();
        let original_home = env::var("HOME").ok();
        let original_userprofile = env::var("USERPROFILE").ok();
        env::set_var("HOME", path);
        env::set_var("USERPROFILE", path);
        Self {
            _lock: lock,
            original_home,
            original_userprofile,
        }
    }
}

impl Drop for AwsHomeEnvGuard {
    fn drop(&mut self) {
        if let Some(home) = &self.original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        if let Some(userprofile) = &self.original_userprofile {
            env::set_var("USERPROFILE", userprofile);
        } else {
            env::remove_var("USERPROFILE");
        }
    }
}

pub fn replay_event(xml_body: &str) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from(xml_body))
            .expect("response"),
    )
}

pub fn ok_event() -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::empty())
            .expect("response"),
    )
}

// --- Error response tests ---

pub fn error_event(status: u16) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(status)
            .body(SdkBody::from(
                r"<Error><Code>AccessDenied</Code><Message>Access Denied</Message></Error>",
            ))
            .expect("response"),
    )
}

#[test]
fn test_virtual_path_parse_local() {
    let path = VirtualPath::parse("/home/user/docs");
    assert!(matches!(path, VirtualPath::Local(_)));
}

#[test]
fn test_virtual_path_parse_s3_bucket() {
    let path = VirtualPath::parse("s3://my-bucket/folder/file.txt");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "default");
            assert_eq!(bucket, Some("my-bucket".to_string()));
            assert_eq!(prefix, "folder/file.txt");
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_parse_s3_with_profile() {
    let path = VirtualPath::parse("s3://myprofile@my-bucket/folder/");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "myprofile");
            assert_eq!(bucket, Some("my-bucket".to_string()));
            assert_eq!(prefix, "folder/");
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_display_formats() {
    let local = VirtualPath::Local(PathBuf::from("/tmp"));
    assert_eq!(local.display(), "/tmp");

    let s3_root = VirtualPath::S3 {
        profile: "default".to_string(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(s3_root.display(), "s3://default@");

    let s3_bucket = VirtualPath::S3 {
        profile: "prod".to_string(),
        bucket: Some("logs".to_string()),
        prefix: String::new(),
    };
    assert_eq!(s3_bucket.display(), "s3://prod@logs");

    let s3_prefix = VirtualPath::S3 {
        profile: "prod".to_string(),
        bucket: Some("logs".to_string()),
        prefix: "2024/".to_string(),
    };
    assert_eq!(s3_prefix.display(), "s3://prod@logs/2024/");
}

#[test]
fn test_list_aws_profiles() {
    // This test just ensures the function doesn't panic
    let profiles = list_aws_profiles();
    // Should at least have "default"
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_virtual_path_display() {
    let local = VirtualPath::Local(PathBuf::from("/test/path"));
    assert_eq!(local.display(), "/test/path");

    let s3_full = VirtualPath::S3 {
        profile: "myprof".to_string(),
        bucket: Some("mybuck".to_string()),
        prefix: "mypre".to_string(),
    };
    assert_eq!(s3_full.display(), "s3://myprof@mybuck/mypre");

    let s3_no_prefix = VirtualPath::S3 {
        profile: "myprof".to_string(),
        bucket: Some("mybuck".to_string()),
        prefix: String::new(),
    };
    assert_eq!(s3_no_prefix.display(), "s3://myprof@mybuck");

    let s3_no_bucket = VirtualPath::S3 {
        profile: "myprof".to_string(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(s3_no_bucket.display(), "s3://myprof@");
}

#[test]
fn test_virtual_path_parse_edge_cases() {
    let path = VirtualPath::parse("s3://");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "default");
            assert_eq!(bucket, None);
            assert!(prefix.is_empty());
        }
        _ => panic!("Expected S3 path"),
    }

    let path = VirtualPath::parse("s3:///");
    assert!(matches!(path, VirtualPath::S3 { .. }));
}

#[test]
fn test_list_aws_profiles_mocked() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let aws_dir = temp_dir.path().join(".aws");
    std::fs::create_dir(&aws_dir).unwrap();
    let creds_file = aws_dir.join("credentials");
    std::fs::write(
        &creds_file,
        "[work]\nregion=us-west-2\n[personal]\nregion=us-east-1",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "work"));
    assert!(profiles.iter().any(|p| p.name == "personal"));
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_s3_client_new_logic() {
    let result = catch_unwind(AssertUnwindSafe(|| S3Client::new("nonexistent_profile")));
    assert!(
        result.is_ok(),
        "S3Client::new should return Result, not panic"
    );
}

#[test]
fn test_list_aws_profiles_reads_credentials() {
    let temp_dir = TempDir::new().expect("temp dir");
    let credentials_dir = temp_dir.path().join(".aws");
    std::fs::create_dir_all(&credentials_dir).expect("create .aws");
    std::fs::write(
        credentials_dir.join("credentials"),
        "[default]\naws_access_key_id=AKIA\n[team]\naws_secret_access_key=SECRET\n",
    )
    .expect("write credentials");
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "default"));
    assert!(profiles.iter().any(|p| p.name == "team"));
}

#[test]
fn test_s3_client_list_buckets_and_objects() -> io::Result<()> {
    let list_buckets_xml = r#"
        <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Buckets>
                <Bucket>
                    <Name>alpha</Name>
                    <CreationDate>2020-01-01T00:00:00.000Z</CreationDate>
                </Bucket>
            </Buckets>
        </ListAllMyBucketsResult>
    "#;
    let list_objects_xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>alpha</Name>
            <Prefix>folder/</Prefix>
            <Delimiter>/</Delimiter>
            <CommonPrefixes>
                <Prefix>folder/subdir/</Prefix>
            </CommonPrefixes>
            <Contents>
                <Key>folder/file.txt</Key>
                <LastModified>2020-01-01T00:00:00.000Z</LastModified>
                <Size>123</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![
        replay_event(list_buckets_xml),
        replay_event(list_objects_xml),
    ]);

    let buckets = client.list_buckets()?;
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].name, "alpha");
    assert!(buckets[0].creation_date.is_some());

    let objects = client.list_objects("alpha", "folder/")?;
    assert!(objects
        .iter()
        .any(|obj| obj.kind.is_dir() && obj.key == "subdir"));
    assert!(objects
        .iter()
        .any(|obj| !obj.kind.is_dir() && obj.key == "file.txt"));
    assert!(objects.iter().any(|obj| obj.last_modified.is_some()));

    Ok(())
}

#[test]
fn test_s3_client_object_operations() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("download.txt");
    let upload_path = temp_dir.path().join("upload.txt");
    std::fs::write(&upload_path, "payload")?;

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
    assert_eq!(std::fs::read_to_string(&download_path)?, "downloaded");

    client.upload_object("alpha", "upload.txt", &upload_path)?;
    client.copy_object("alpha", "upload.txt", "alpha", "copied.txt")?;
    client.delete_object("alpha", "copied.txt")?;

    Ok(())
}

#[test]
fn test_list_buckets_error_response() {
    let client = build_test_client(vec![error_event(403)]);
    let result = client.list_buckets();
    assert!(result.is_err());
}

#[test]
fn test_list_objects_error_response() {
    let client = build_test_client(vec![error_event(404)]);
    let result = client.list_objects("nonexistent-bucket", "");
    assert!(result.is_err());
}

#[test]
fn test_download_object_error_response() {
    let client = build_test_client(vec![error_event(403)]);
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("out.txt");
    let result = client.download_object("bucket", "key", &path);
    assert!(result.is_err());
}

fn get_event(bucket: &str, key: &str, body: &str) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri(format!("https://s3.us-east-1.amazonaws.com/{bucket}/{key}"))
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from(body.to_string()))
            .expect("response"),
    )
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
    assert!(
        leftovers.is_empty(),
        "temp files left behind: {leftovers:?}"
    );
    assert_eq!(std::fs::read_to_string(&download_path)?, "content");
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

    assert_eq!(std::fs::read_to_string(&download_path)?, "downloaded");
    assert_eq!(std::fs::read_to_string(&bystander)?, "do not touch");
    Ok(())
}

/// A destination that itself ends in `.part` downloads normally and leaves no
/// staging file behind.
///
/// HONEST SCOPE — this is deliberately weaker than its name once suggested. The
/// old `with_extension("part")` staging path *equalled* such a destination, so
/// the write stopped being atomic and the error cleanup would have deleted the
/// destination; but on the success path `with_extension("part")` is a no-op here
/// and `rename(x, x)` succeeds, so no assertion about the final content can tell
/// the two implementations apart. Observing the non-atomic window needs a write
/// fault *after* a successful GET, which the static replay harness cannot
/// inject. What is pinned instead: the payload lands, and the directory holds
/// nothing but the destination — a staging file named after the destination
/// would show up as a leftover here, and the sibling
/// `test_download_leaves_unrelated_part_file_alone` pins the collision half of
/// the same bug with a test that genuinely goes red when the fix is reverted.
#[test]
fn test_download_to_a_part_destination_leaves_no_staging_file() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("archive.part");

    let client = build_test_client(vec![get_event("bucket", "archive.part", "payload")]);
    client.download_object("bucket", "archive.part", &download_path)?;

    assert_eq!(std::fs::read_to_string(&download_path)?, "payload");
    let leftovers: Vec<PathBuf> = fs::read_dir(temp_dir.path())?
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path != &download_path)
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files left behind: {leftovers:?}"
    );
    Ok(())
}

#[test]
fn test_download_preserves_existing_file_on_failure() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("temp dir");
    let download_path = temp_dir.path().join("existing.txt");
    std::fs::write(&download_path, "original")?;

    let client = build_test_client(vec![error_event(500)]);
    let result = client.download_object("bucket", "existing.txt", &download_path);
    assert!(result.is_err());

    // Original file should be preserved
    assert_eq!(std::fs::read_to_string(&download_path)?, "original");
    Ok(())
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
