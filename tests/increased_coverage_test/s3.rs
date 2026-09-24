//! Purpose: S3 virtual paths, AWS profiles and the S3 drive entry.
#![cfg(feature = "s3")]

use super::*;

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_and_display_variants() {
    assert_eq!(
        VirtualPath::parse("/tmp/local"),
        VirtualPath::Local(PathBuf::from("/tmp/local"))
    );

    assert_eq!(
        VirtualPath::parse("s3://bucket-name/prefix").display(),
        "s3://default@bucket-name/prefix"
    );
    assert_eq!(
        VirtualPath::parse("s3://prod@bucket-name/prefix").display(),
        "s3://prod@bucket-name/prefix"
    );
    assert_eq!(VirtualPath::parse("s3://").display(), "s3://default@");
    assert_eq!(VirtualPath::parse("s3://prod@").display(), "s3://prod@");
}

#[cfg(feature = "s3")]
#[test]
fn test_list_aws_profiles_always_contains_default() {
    let profiles = list_aws_profiles();
    assert!(!profiles.is_empty());
    assert_eq!(profiles[0].name, "default");
    assert!(profiles.iter().any(|profile| profile.name == "default"));
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_bucket_without_prefix() {
    assert_eq!(
        VirtualPath::parse("s3://bucket-name").display(),
        "s3://default@bucket-name"
    );
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_bucket_with_empty_bucket() {
    assert_eq!(
        VirtualPath::parse("s3://prod@/prefix").display(),
        "s3://prod@"
    );
}

#[cfg(feature = "s3")]
#[test]
fn test_virtual_path_parse_profile_bucket_without_prefix() {
    assert_eq!(
        VirtualPath::parse("s3://prod@bucket-name").display(),
        "s3://prod@bucket-name"
    );
}

#[cfg(feature = "s3")]
#[test]
fn test_app_get_all_drives_contains_default_s3_profile() {
    let all = App::get_all_drives();
    assert!(all.iter().any(|entry| matches!(
        entry,
        DriveEntry::S3Profile(name) if name == "default"
    )));
}
