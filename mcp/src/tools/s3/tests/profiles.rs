use super::fixture::{self, response, Fixture};
use crate::tools::s3::{make_s3_client, S3ProfileParam};
use rmcp::{handler::server::wrapper::Parameters, model::ErrorCode};

#[test]
fn invalid_profiles_are_refused_before_loading_credentials() {
    fixture::run(async {
        for profile in [
            String::new(),
            "x".repeat(129),
            "a/b".into(),
            "a\\b".into(),
            "a\nb".into(),
            "a..b".into(),
        ] {
            let error = make_s3_client(&profile).await.unwrap_err();
            assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
            assert_eq!(error.message, "invalid AWS profile name");
        }
    });
}

#[test]
fn buckets_keep_optional_metadata_and_use_the_selected_profile() {
    fixture::run(async {
        let fixture = Fixture::new(vec![response(200, "<ListAllMyBucketsResult><Buckets><Bucket><Name>archive</Name><CreationDate>2024-01-02T03:04:05Z</CreationDate></Bucket><Bucket/></Buckets></ListAllMyBucketsResult>")]);
        let result = fixture
            .server()
            .s3_list_buckets(Parameters(S3ProfileParam {
                profile: "fixture".into(),
            }))
            .await
            .unwrap();
        let value = fixture::json(&result);
        assert_eq!(value["profile"], "fixture");
        assert_eq!(value["count"], 2);
        assert_eq!(value["buckets"][0]["name"], "archive");
        assert!(value["buckets"][0]["creation_date"]
            .as_str()
            .unwrap()
            .contains("2024-01-02"));
        assert_eq!(value["buckets"][1]["name"], "");
        assert!(value["buckets"][1]["creation_date"].is_null());
        assert!(fixture.requests()[0].headers.starts_with("GET /"));
    });
}

#[test]
fn bucket_service_errors_are_returned_to_the_tool_caller() {
    fixture::run(async {
        let fixture = Fixture::new(vec![response(
            403,
            "<Error><Code>AccessDenied</Code><Message>fixture denied</Message></Error>",
        )]);
        let error = fixture
            .server()
            .s3_list_buckets(Parameters(S3ProfileParam {
                profile: "fixture".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert!(!error.message.is_empty());
        assert_eq!(fixture.requests().len(), 1);
    });
}
