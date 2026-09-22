//! Unit tests: virtual path (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::EntryKind;
use crate::models::FileSize;

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

use tempfile::TempDir;

#[test]
fn test_s3_object_folder() {
    let folder = S3Object {
        key: "images".to_string(),
        size: FileSize::from_bytes(0),
        kind: EntryKind::Directory,
        last_modified: None,
    };
    assert!(folder.kind.is_dir());
    assert_eq!(folder.size, FileSize::from_bytes(0));
    assert!(folder.last_modified.is_none());
}

// --- S3Client field access tests ---

#[test]
fn test_s3_client_fields() {
    let client = build_test_client(vec![]);
    assert_eq!(client.profile, "test");
    assert!(client.current_bucket.is_none());
}

#[test]
fn test_virtual_path_parse_profile_with_empty_bucket_and_prefix() {
    let path = VirtualPath::parse("s3://prod@/nested/path");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "prod");
            assert_eq!(bucket, None);
            assert_eq!(prefix, "nested/path");
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_display_no_bucket_keeps_profile_marker() {
    let path = VirtualPath::S3 {
        profile: "staging".to_string(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(path.display(), "s3://staging@");
}

#[test]
fn test_virtual_path_display_local_unicode_path() {
    let path = VirtualPath::Local(PathBuf::from("./данные/файл.txt"));
    assert_eq!(path.display(), "./данные/файл.txt");
}

#[test]
fn test_list_aws_profiles_deduplicates_default_section() {
    let temp_dir = TempDir::new().expect("test setup");
    let aws_dir = temp_dir.path().join(".aws");
    fs::create_dir_all(&aws_dir).expect("test setup");
    fs::write(
        aws_dir.join("credentials"),
        "[default]\naws_access_key_id = a\n[prod]\naws_access_key_id = b\n",
    )
    .expect("test setup");

    let _guard = AwsHomeEnvGuard::set(temp_dir.path());
    let profiles = list_aws_profiles();
    let default_count = profiles.iter().filter(|p| p.name == "default").count();
    assert_eq!(default_count, 1);
    assert!(profiles.iter().any(|p| p.name == "prod"));
}

#[test]
fn test_s3_client_with_bucket_and_prefix() {
    let http_client = StaticReplayClient::new(vec![]);
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
        .region(Region::new("eu-west-1"))
        .http_client(http_client)
        .behavior_version_latest()
        .build();
    let client_inner = Client::from_conf(config);
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let client = S3Client {
        client: client_inner,
        runtime,
        profile: "production".to_string(),
        current_bucket: Some("my-data-bucket".to_string()),
        current_prefix: "logs/2024/".to_string(),
    };
    assert_eq!(client.profile, "production");
    assert_eq!(client.current_bucket, Some("my-data-bucket".to_string()));
    assert_eq!(client.current_prefix, "logs/2024/");
}

// --- list_aws_profiles edge cases ---

#[test]
fn test_list_aws_profiles_no_credentials_file() {
    let temp_dir = TempDir::new().expect("temp dir");
    // No .aws directory at all
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // Should still have "default" as fallback
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "default");
}

#[test]
fn test_list_aws_profiles_empty_credentials_file() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    std::fs::create_dir(&aws_dir).unwrap();
    std::fs::write(aws_dir.join("credentials"), "").unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // Should have "default" inserted
    assert!(profiles.iter().any(|p| p.name == "default"));
}

#[test]
fn test_list_aws_profiles_default_already_present() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    std::fs::create_dir(&aws_dir).unwrap();
    std::fs::write(
        aws_dir.join("credentials"),
        "[default]\naws_access_key_id=AKIA123\n[staging]\naws_access_key_id=AKIA456\n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    // "default" should appear exactly once
    let default_count = profiles.iter().filter(|p| p.name == "default").count();
    assert_eq!(default_count, 1);
    assert!(profiles.iter().any(|p| p.name == "staging"));
}

#[test]
fn test_list_aws_profiles_malformed_lines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    std::fs::create_dir(&aws_dir).unwrap();
    std::fs::write(
        aws_dir.join("credentials"),
        "not a section\n[valid]\nkey=value\n[incomplete\nno-bracket]\n  [spaced]  \n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profiles = list_aws_profiles();
    assert!(profiles.iter().any(|p| p.name == "valid"));
    // "[spaced]" after trim() becomes "[spaced]" which starts with [ and ends with ]
    assert!(profiles.iter().any(|p| p.name == "spaced"));
    // "[incomplete" doesn't end with ], should not appear
    assert!(!profiles.iter().any(|p| p.name == "incomplete"));
    // "no-bracket]" doesn't start with [, should not appear
    assert!(!profiles.iter().any(|p| p.name == "no-bracket"));
}

#[test]
fn test_list_aws_profiles_preserves_section_order_after_default() {
    let temp_dir = TempDir::new().expect("temp dir");
    let aws_dir = temp_dir.path().join(".aws");
    std::fs::create_dir(&aws_dir).unwrap();
    std::fs::write(
        aws_dir.join("credentials"),
        "[work]\naws_access_key_id=1\n[personal]\naws_access_key_id=2\n",
    )
    .unwrap();
    let _env = AwsHomeEnvGuard::set(temp_dir.path());

    let profile_names: Vec<String> = list_aws_profiles()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(profile_names, vec!["default", "work", "personal"]);
}

// --- Download with binary data ---

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
    let written = std::fs::read(&download_path)?;
    assert_eq!(written, binary_data);
    Ok(())
}

// --- Copy source format test ---

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
