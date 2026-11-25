//! Purpose: unit tests for `S3Client::list_buckets` on full, empty and failing responses.

use super::*;

#[test]
fn test_s3_client_list_buckets_and_objects() -> io::Result<()> {
    let list_buckets_xml = TEST_S3_CLIENT_LIST_BUCKETS_AND_OBJECTS_LIST_BUCKETS_XML;
    let list_objects_xml = TEST_S3_CLIENT_LIST_BUCKETS_AND_OBJECTS_LIST_OBJECTS_XML;
    let client = build_test_client(vec![
        replay_event(list_buckets_xml),
        replay_event(list_objects_xml),
    ]);

    let buckets = client.list_buckets()?;
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].name, "alpha");
    assert!(buckets[0].creation_date.is_some());

    let objects = client.list_objects("alpha", "folder/")?;
    assert!(objects
        .iter()
        .any(|obj| obj.kind.is_dir() && obj.key == "subdir"));
    assert!(objects
        .iter()
        .any(|obj| !obj.kind.is_dir() && obj.key == "file.txt"));
    assert!(objects.iter().any(|obj| obj.last_modified.is_some()));

    Ok(())
}

#[test]
fn test_list_buckets_error_response() {
    let client = build_test_client(vec![error_event(403)]);
    let result = client.list_buckets();
    assert!(result.is_err());
}

#[test]
fn test_list_buckets_empty() -> io::Result<()> {
    let xml = r#"
        <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Buckets/>
        </ListAllMyBucketsResult>
    "#;
    let client = build_test_client(vec![replay_event(xml)]);
    let buckets = client.list_buckets()?;
    assert_eq!(buckets.len(), 0);
    Ok(())
}

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

const TEST_S3_CLIENT_LIST_BUCKETS_AND_OBJECTS_LIST_BUCKETS_XML: &str = r#"
        <ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Buckets>
                <Bucket>
                    <Name>alpha</Name>
                    <CreationDate>2020-01-01T00:00:00.000Z</CreationDate>
                </Bucket>
            </Buckets>
        </ListAllMyBucketsResult>
    "#;

const TEST_S3_CLIENT_LIST_BUCKETS_AND_OBJECTS_LIST_OBJECTS_XML: &str = r#"
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
            <Name>alpha</Name>
            <Prefix>folder/</Prefix>
            <Delimiter>/</Delimiter>
            <CommonPrefixes>
                <Prefix>folder/subdir/</Prefix>
            </CommonPrefixes>
            <Contents>
                <Key>folder/file.txt</Key>
                <LastModified>2020-01-01T00:00:00.000Z</LastModified>
                <Size>123</Size>
            </Contents>
        </ListBucketResult>
    "#;
