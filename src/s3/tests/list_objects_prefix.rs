//! Purpose: unit tests for how `S3Client::list_objects` trims the folder prefix off each row.

use super::*;

use crate::models::FileSize;

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
