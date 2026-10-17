//! Purpose: unit tests for `VirtualPath::parse` on local, bucket, profile and edge-case inputs.

use super::*;

use std::path::PathBuf;

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
            assert_eq!(bucket, Some("my-bucket".to_owned()));
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
            assert_eq!(bucket, Some("my-bucket".to_owned()));
            assert_eq!(prefix, "folder/");
        }
        _ => panic!("Expected S3 path"),
    }
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
fn test_virtual_path_parse_bucket_only_no_slash() {
    let path = VirtualPath::parse("s3://my-bucket");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "default");
            assert_eq!(bucket, Some("my-bucket".to_owned()));
            assert!(prefix.is_empty());
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_parse_profile_at_bucket_no_prefix() {
    let path = VirtualPath::parse("s3://prod@my-bucket");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "prod");
            assert_eq!(bucket, Some("my-bucket".to_owned()));
            assert!(prefix.is_empty());
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_parse_profile_without_bucket() {
    let path = VirtualPath::parse("s3://staging@");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "staging");
            assert_eq!(bucket, None);
            assert!(prefix.is_empty());
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_parse_s3_triple_slash_details() {
    // s3:/// -> rest="" -> parts=[""] -> empty bucket
    let path = VirtualPath::parse("s3:///");
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
}

#[test]
fn test_virtual_path_parse_deep_prefix() {
    let path = VirtualPath::parse("s3://staging@archive/2024/01/15/logs/app.log");
    match path {
        VirtualPath::S3 {
            profile,
            bucket,
            prefix,
        } => {
            assert_eq!(profile, "staging");
            assert_eq!(bucket, Some("archive".to_owned()));
            assert_eq!(prefix, "2024/01/15/logs/app.log");
        }
        _ => panic!("Expected S3 path"),
    }
}

#[test]
fn test_virtual_path_parse_relative_local() {
    let path = VirtualPath::parse("./relative/path");
    match path {
        VirtualPath::Local(p) => assert_eq!(p, PathBuf::from("./relative/path")),
        _ => panic!("Expected local path"),
    }
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
