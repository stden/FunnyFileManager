//! Purpose: Displaying local and S3 virtual paths.
#![cfg(feature = "s3")]

use super::*;

// ═══════════════════════════════════════════════════════════════════
// VirtualPath display variants
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_local_display() {
    let vp = VirtualPath::parse("/home/user/docs");
    assert_eq!(vp.display(), "/home/user/docs");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_s3_with_bucket_no_prefix() {
    let vp = VirtualPath::parse("s3://mybucket");
    assert!(matches!(vp, VirtualPath::S3 { .. }));
    let display = vp.display();
    assert!(display.contains("mybucket"));
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_s3_with_profile_no_bucket() {
    let vp = VirtualPath::parse("s3://myprofile@");
    if let VirtualPath::S3 {
        profile, bucket, ..
    } = &vp
    {
        assert_eq!(profile, "myprofile");
        assert!(bucket.is_none());
    }
    let display = vp.display();
    assert!(display.contains("myprofile"));
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_s3_with_all_parts() {
    let vp = VirtualPath::parse("s3://prod@bucket/path/to/obj");
    if let VirtualPath::S3 {
        profile,
        bucket,
        prefix,
    } = &vp
    {
        assert_eq!(profile, "prod");
        assert_eq!(bucket.as_deref(), Some("bucket"));
        assert_eq!(prefix, "path/to/obj");
    }
    let display = vp.display();
    assert_eq!(display, "s3://prod@bucket/path/to/obj");
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_empty_s3() {
    let vp = VirtualPath::parse("s3://");
    assert!(matches!(vp, VirtualPath::S3 { .. }));
}
