//! Purpose: unit tests for the plain value types `AwsProfile`, `S3Bucket` and `S3Object`.

use super::*;

use crate::models::{EntryKind, FileSize};
use std::time::SystemTime;

#[test]
fn test_aws_profile_clone_debug() {
    let profile = AwsProfile {
        name: "prod".to_owned(),
        region: Some("eu-west-1".to_owned()),
    };
    let cloned = profile.clone();
    assert_eq!(cloned.name, "prod");
    assert_eq!(cloned.region, Some("eu-west-1".to_owned()));
    let debug = format!("{profile:?}");
    assert!(debug.contains("prod"));
}

#[test]
fn test_aws_profile_no_region() {
    let profile = AwsProfile {
        name: "test".to_owned(),
        region: None,
    };
    assert_eq!(profile.name, "test");
    assert!(profile.region.is_none());
}

#[test]
fn test_s3_bucket_clone_debug() {
    let bucket = S3Bucket {
        name: "my-data".to_owned(),
        creation_date: Some("2024-01-01".to_owned()),
    };
    let cloned = bucket.clone();
    assert_eq!(cloned.name, "my-data");
    assert_eq!(cloned.creation_date, Some("2024-01-01".to_owned()));
    let debug = format!("{bucket:?}");
    assert!(debug.contains("my-data"));
}

#[test]
fn test_s3_bucket_no_creation_date() {
    let bucket = S3Bucket {
        name: "temp".to_owned(),
        creation_date: None,
    };
    assert!(bucket.creation_date.is_none());
}

#[test]
fn test_s3_object_clone_debug() {
    let obj = S3Object {
        key: "file.txt".to_owned(),
        size: FileSize::from_bytes(1024),
        kind: EntryKind::File,
        last_modified: Some(SystemTime::UNIX_EPOCH),
    };
    let cloned = obj.clone();
    assert_eq!(cloned.key, "file.txt");
    assert_eq!(cloned.size, FileSize::from_bytes(1024));
    assert!(!cloned.kind.is_dir());
    assert!(cloned.last_modified.is_some());
    let debug = format!("{obj:?}");
    assert!(debug.contains("file.txt"));
}

#[test]
fn test_s3_object_folder() {
    let folder = S3Object {
        key: "images".to_owned(),
        size: FileSize::from_bytes(0),
        kind: EntryKind::Directory,
        last_modified: None,
    };
    assert!(folder.kind.is_dir());
    assert_eq!(folder.size, FileSize::from_bytes(0));
    assert!(folder.last_modified.is_none());
}
