//! Purpose: Displaying local and S3 virtual paths.
#![cfg(feature = "s3")]

use super::*;

// ─── S3 VirtualPath edge cases (s3.rs) ──────────────────────────────

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_local() {
    let vp = VirtualPath::Local(PathBuf::from("/tmp/test"));
    assert_eq!(vp.display(), "/tmp/test");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_with_bucket_and_prefix() {
    let vp = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("my-bucket".to_owned()),
        prefix: "path/to/file".to_owned(),
    };
    assert_eq!(vp.display(), "s3://prod@my-bucket/path/to/file");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_no_bucket() {
    let vp = VirtualPath::S3 {
        profile: "dev".to_owned(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(vp.display(), "s3://dev@");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_display_s3_bucket_no_prefix() {
    let vp = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("bucket".to_owned()),
        prefix: String::new(),
    };
    assert_eq!(vp.display(), "s3://prod@bucket");
}
