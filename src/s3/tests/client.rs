//! Purpose: unit tests for building an `S3Client` and the fields it carries.

use super::*;

#[test]
fn test_s3_client_new_logic() {
    let result = catch_unwind(AssertUnwindSafe(|| S3Client::new("nonexistent_profile")));
    assert!(
        result.is_ok(),
        "S3Client::new should return Result, not panic"
    );
}

#[test]
fn test_s3_client_fields() {
    let client = build_test_client(vec![]);
    assert_eq!(client.profile, "test");
    assert!(client.current_bucket.is_none());
}

#[test]
fn test_s3_client_with_bucket_and_prefix() {
    let http_client = StaticReplayClient::new(vec![]);
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
        .region(Region::new("eu-west-1"))
        .http_client(http_client)
        .behavior_version_latest()
        .build();
    let client_inner = Client::from_conf(config);
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let client = S3Client {
        client: client_inner,
        runtime,
        profile: "production".to_owned(),
        current_bucket: Some("my-data-bucket".to_owned()),
        current_prefix: "logs/2024/".to_owned(),
    };
    assert_eq!(client.profile, "production");
    assert_eq!(client.current_bucket, Some("my-data-bucket".to_owned()));
    assert_eq!(client.current_prefix, "logs/2024/");
}
