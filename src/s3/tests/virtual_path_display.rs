//! Purpose: unit tests for the `VirtualPath` display form, equality and `Debug`.

use super::*;

use std::path::PathBuf;

#[test]
fn test_virtual_path_display_formats() {
    let local = VirtualPath::Local(PathBuf::from("/tmp"));
    assert_eq!(local.display(), "/tmp");

    let s3_root = VirtualPath::S3 {
        profile: "default".to_owned(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(s3_root.display(), "s3://default@");

    let s3_bucket = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("logs".to_owned()),
        prefix: String::new(),
    };
    assert_eq!(s3_bucket.display(), "s3://prod@logs");

    let s3_prefix = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("logs".to_owned()),
        prefix: "2024/".to_owned(),
    };
    assert_eq!(s3_prefix.display(), "s3://prod@logs/2024/");
}

#[test]
fn test_virtual_path_display() {
    let local = VirtualPath::Local(PathBuf::from("/test/path"));
    assert_eq!(local.display(), "/test/path");

    let s3_full = VirtualPath::S3 {
        profile: "myprof".to_owned(),
        bucket: Some("mybuck".to_owned()),
        prefix: "mypre".to_owned(),
    };
    assert_eq!(s3_full.display(), "s3://myprof@mybuck/mypre");

    let s3_no_prefix = VirtualPath::S3 {
        profile: "myprof".to_owned(),
        bucket: Some("mybuck".to_owned()),
        prefix: String::new(),
    };
    assert_eq!(s3_no_prefix.display(), "s3://myprof@mybuck");

    let s3_no_bucket = VirtualPath::S3 {
        profile: "myprof".to_owned(),
        bucket: None,
        prefix: String::new(),
    };
    assert_eq!(s3_no_bucket.display(), "s3://myprof@");
}

#[test]
fn test_virtual_path_display_no_bucket_keeps_profile_marker() {
    let path = VirtualPath::S3 {
        profile: "staging".to_owned(),
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
fn test_virtual_path_equality() {
    let a = VirtualPath::parse("s3://prod@bucket/prefix");
    let b = VirtualPath::S3 {
        profile: "prod".to_owned(),
        bucket: Some("bucket".to_owned()),
        prefix: "prefix".to_owned(),
    };
    assert_eq!(a, b);

    let c = VirtualPath::Local(PathBuf::from("/tmp"));
    let d = VirtualPath::Local(PathBuf::from("/tmp"));
    assert_eq!(c, d);

    assert_ne!(a, c);
}

#[test]
fn test_virtual_path_clone_and_debug() {
    let path = VirtualPath::parse("s3://test@bucket/key");
    let cloned = path.clone();
    assert_eq!(path, cloned);
    let debug_str = format!("{path:?}");
    assert!(debug_str.contains("S3"));
    assert!(debug_str.contains("test"));
}
