//! Purpose: unit tests for `S3Client::list_objects`: pages, row fields and failures.

use super::*;

use crate::models::FileSize;
use crate::s3::listing::file_from_object;

#[test]
fn test_list_objects_error_response() {
    let client = build_test_client(vec![error_event(404)]);
    let result = client.list_objects("nonexistent-bucket", "");
    assert!(result.is_err());
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
