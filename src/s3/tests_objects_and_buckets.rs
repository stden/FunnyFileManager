//! Unit tests: objects and buckets (split out to keep files under 500 lines).

use super::tests::*;
use crate::models::EntryKind;
use crate::models::FileSize;
use std::time::SystemTime;

use super::*;

use std::path::PathBuf;

use tempfile::TempDir;

#[test]
fn test_upload_object_error_response() {
    let temp_dir = TempDir::new().expect("temp dir");
    let upload_path = temp_dir.path().join("file.txt");
    std::fs::write(&upload_path, "data").unwrap();
    let client = build_test_client(vec![error_event(500)]);
    let result = client.upload_object("bucket", "key", &upload_path);
    assert!(result.is_err());
}

#[test]
fn test_upload_nonexistent_file() {
    let client = build_test_client(vec![ok_event()]);
    let bad_path = PathBuf::from("/nonexistent/path/file.txt");
    let result = client.upload_object("bucket", "key", &bad_path);
    assert!(result.is_err());
}

// --- Empty results tests ---

#[test]
fn test_list_buckets_empty() -> io::Result<()> {
    let xml = r#"
        <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Buckets/>
        </ListAllMyBucketsResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let buckets = client.list_buckets()?;
    assert!(buckets.is_empty());
    Ok(())
}

#[test]
fn test_list_objects_empty() -> io::Result<()> {
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix></Prefix>
            <Delimiter>/</Delimiter>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "")?;
    assert!(objects.is_empty());
    Ok(())
}

// --- list_objects edge cases ---

#[test]
fn test_list_objects_empty_prefix() -> io::Result<()> {
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix></Prefix>
            <Delimiter>/</Delimiter>
            <CommonPrefixes>
                <Prefix>docs/</Prefix>
            </CommonPrefixes>
            <Contents>
                <Key>readme.txt</Key>
                <Size>42</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "")?;
    assert_eq!(objects.len(), 2);
    assert!(objects.iter().any(|o| o.kind.is_dir() && o.key == "docs"));
    assert!(objects
        .iter()
        .any(|o| !o.kind.is_dir() && o.key == "readme.txt" && o.size == FileSize::from_bytes(42)));
    Ok(())
}

#[test]
fn test_list_objects_skips_prefix_itself() -> io::Result<()> {
    // When key == prefix, it should be skipped (folder marker)
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix>folder/</Prefix>
            <Delimiter>/</Delimiter>
            <Contents>
                <Key>folder/</Key>
                <Size>0</Size>
            </Contents>
            <Contents>
                <Key>folder/real-file.txt</Key>
                <Size>100</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "folder/")?;
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].key, "real-file.txt");
    Ok(())
}

#[test]
fn test_list_objects_strips_a_prefix_without_a_trailing_slash() -> io::Result<()> {
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix>data/</Prefix>
            <Delimiter>/</Delimiter>
            <Contents>
                <Key>data/notes.txt</Key>
                <Size>4</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "data")?;
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].key, "notes.txt");
    Ok(())
}

#[test]
fn test_list_objects_follows_the_continuation_token() -> io::Result<()> {
    let first = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <IsTruncated>true</IsTruncated>
            <NextContinuationToken>page-2</NextContinuationToken>
            <Contents>
                <Key>a.txt</Key>
                <Size>1</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let second = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <IsTruncated>false</IsTruncated>
            <Contents>
                <Key>b.txt</Key>
                <Size>2</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(first), replay_event(second)]);
    let objects = client.list_objects("mybucket", "")?;
    let names: Vec<_> = objects.iter().map(|object| object.key.as_str()).collect();
    assert_eq!(names, ["a.txt", "b.txt"]);
    Ok(())
}

#[test]
fn test_list_objects_skips_slash_name() -> io::Result<()> {
    // After stripping prefix, if name is "/" it should be skipped
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix>data</Prefix>
            <Delimiter>/</Delimiter>
            <Contents>
                <Key>data/</Key>
                <Size>0</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "data")?;
    assert!(objects.is_empty());
    Ok(())
}

#[test]
fn test_list_objects_no_last_modified_no_size() -> io::Result<()> {
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix></Prefix>
            <Delimiter>/</Delimiter>
            <Contents>
                <Key>orphan.bin</Key>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "")?;
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].size, FileSize::from_bytes(0));
    assert!(objects[0].last_modified.is_none());
    Ok(())
}

#[test]
fn test_list_objects_multiple_folders_and_files() -> io::Result<()> {
    let xml = TEST_LIST_OBJECTS_MULTIPLE_FOLDERS_AND_FILES_XML;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "proj/")?;
    assert_eq!(objects.len(), 4);
    let folders: Vec<_> = objects.iter().filter(|o| o.kind.is_dir()).collect();
    let files: Vec<_> = objects.iter().filter(|o| !o.kind.is_dir()).collect();
    assert_eq!(folders.len(), 2);
    assert_eq!(files.len(), 2);
    assert!(folders.iter().any(|f| f.key == "src"));
    assert!(folders.iter().any(|f| f.key == "tests"));
    assert!(files
        .iter()
        .any(|f| f.key == "Cargo.toml" && f.size == FileSize::from_bytes(500)));
    assert!(files
        .iter()
        .any(|f| f.key == "README.md" && f.size == FileSize::from_bytes(200)));
    Ok(())
}

#[test]
fn test_list_objects_skips_empty_folder_name_after_prefix_trim() -> io::Result<()> {
    let xml = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix>proj/</Prefix>
            <Delimiter>/</Delimiter>
            <CommonPrefixes>
                <Prefix>proj/</Prefix>
            </CommonPrefixes>
            <Contents>
                <Key>proj/file.txt</Key>
                <Size>7</Size>
            </Contents>
        </ListBucketResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let objects = client.list_objects("mybucket", "proj/")?;
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].key, "file.txt");
    assert!(!objects[0].kind.is_dir());
    Ok(())
}

// --- Multiple buckets test ---

#[test]
fn test_list_multiple_buckets() -> io::Result<()> {
    let xml = r#"
        <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Buckets>
                <Bucket>
                    <Name>logs</Name>
                    <CreationDate>2023-01-01T00:00:00.000Z</CreationDate>
                </Bucket>
                <Bucket>
                    <Name>backups</Name>
                </Bucket>
                <Bucket>
                    <Name>data</Name>
                    <CreationDate>2024-06-15T10:30:00.000Z</CreationDate>
                </Bucket>
            </Buckets>
        </ListAllMyBucketsResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let buckets = client.list_buckets()?;
    assert_eq!(buckets.len(), 3);
    assert_eq!(buckets[0].name, "logs");
    assert!(buckets[0].creation_date.is_some());
    assert_eq!(buckets[1].name, "backups");
    assert!(buckets[1].creation_date.is_none());
    assert_eq!(buckets[2].name, "data");
    Ok(())
}

// --- VirtualPath additional edge cases ---

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

// --- Struct trait tests ---

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

const TEST_LIST_OBJECTS_MULTIPLE_FOLDERS_AND_FILES_XML: &str = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>mybucket</Name>
            <Prefix>proj/</Prefix>
            <Delimiter>/</Delimiter>
            <CommonPrefixes>
                <Prefix>proj/src/</Prefix>
            </CommonPrefixes>
            <CommonPrefixes>
                <Prefix>proj/tests/</Prefix>
            </CommonPrefixes>
            <Contents>
                <Key>proj/Cargo.toml</Key>
                <Size>500</Size>
                <LastModified>2024-06-01T12:00:00.000Z</LastModified>
            </Contents>
            <Contents>
                <Key>proj/README.md</Key>
                <Size>200</Size>
            </Contents>
        </ListBucketResult>
    "#;

/// S3 reports sizes as `i64`: a negative one is malformed and shows as empty,
/// and the largest positive one converts exactly.
#[test]
fn test_object_size_conversion_boundaries() {
    let size_of = |bytes: i64| {
        let object = aws_sdk_s3::types::Object::builder()
            .key("file.bin")
            .size(bytes)
            .build();
        file_from_object(&object, "").map(|row| row.size)
    };
    assert_eq!(size_of(-1), Some(FileSize::from_bytes(0)));
    assert_eq!(size_of(i64::MIN), Some(FileSize::from_bytes(0)));
    assert_eq!(size_of(0), Some(FileSize::from_bytes(0)));
    assert_eq!(
        size_of(i64::MAX),
        Some(FileSize::from_bytes(i64::MAX.unsigned_abs()))
    );
}
