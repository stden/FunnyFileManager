//! Purpose: Parsing and round-tripping S3 and local virtual paths.
#![cfg(feature = "s3")]

use super::*;

// ---------------------------------------------------------------------------
// 3. VirtualPath::parse — profile@empty-bucket yields None bucket
// ---------------------------------------------------------------------------

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_at_empty_bucket() {
    let path = VirtualPath::parse("s3://myprofile@");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "myprofile");
            assert_eq!(bucket, None);
            assert_eq!(prefix, "");
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_at_empty_bucket_with_prefix() {
    let path = VirtualPath::parse("s3://myprofile@/some/prefix");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "myprofile");
            // bucket_part is "myprofile@", split_once('/') gives ("myprofile@", "some/prefix")
            // then profile@bucket_name split: profile="myprofile", bucket_name=""
            assert_eq!(bucket, None);
            assert_eq!(prefix, "some/prefix");
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_bare_s3_no_trailing_slash() {
    // "s3://" with nothing after — tests the empty bucket_part branch
    let path = VirtualPath::parse("s3://");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "default");
            assert_eq!(bucket, None);
            assert_eq!(prefix, "");
        }
        VirtualPath::Local(_) => panic!("Expected S3 path"),
    }
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_roundtrip_local() {
    let original = "/some/path/here";
    let parsed = VirtualPath::parse(original);
    assert_eq!(parsed.display(), original);
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_roundtrip_s3_with_prefix() {
    let parsed = VirtualPath::parse("s3://prod@bucket/dir/subdir");
    assert_eq!(parsed.display(), "s3://prod@bucket/dir/subdir");
}
