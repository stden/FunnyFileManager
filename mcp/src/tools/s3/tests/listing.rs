use super::fixture::{self, response, Fixture};
use crate::tools::s3::S3ListObjectsParams;
use rmcp::{handler::server::wrapper::Parameters, model::ErrorCode};

fn params(prefix: Option<&str>) -> Parameters<S3ListObjectsParams> {
    Parameters(S3ListObjectsParams {
        profile: "fixture".into(),
        bucket: "bucket".into(),
        prefix: prefix.map(str::to_owned),
    })
}

#[test]
fn pages_preserve_folders_objects_and_optional_metadata() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            response(200, "<ListBucketResult><IsTruncated>true</IsTruncated><NextContinuationToken>page2</NextContinuationToken><CommonPrefixes><Prefix>folder/nested/</Prefix></CommonPrefixes><CommonPrefixes/><Contents><Key>folder/file</Key><Size>8</Size><LastModified>2024-01-02T03:04:05Z</LastModified></Contents></ListBucketResult>"),
            response(200, "<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Size>-7</Size></Contents><Contents><Key>folder/unknown</Key></Contents></ListBucketResult>"),
        ]);
        let value = fixture::json(
            &fixture
                .server()
                .s3_list_objects(params(Some("folder")))
                .await
                .unwrap(),
        );
        assert_eq!(value["prefix"], "folder");
        assert_eq!(value["count"], 4);
        assert_eq!(value["objects"][0]["type"], "folder");
        assert_eq!(value["objects"][1]["key"], "folder/file");
        assert_eq!(value["objects"][1]["size"], 8);
        assert!(value["objects"][1]["last_modified"]
            .as_str()
            .unwrap()
            .contains("2024-01-02"));
        assert_eq!(value["objects"][2]["key"], "");
        assert_eq!(value["objects"][2]["size"], 0);
        assert_eq!(value["objects"][3]["size"], 0);
        let requests = fixture.requests();
        assert!(requests[0].headers.contains("prefix=folder%2F"));
        assert!(requests[0].headers.contains("delimiter=%2F"));
        assert!(requests[1].headers.contains("continuation-token=page2"));
    });
}

#[test]
fn empty_and_already_terminated_prefixes_do_not_gain_extra_slashes() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            response(200, "<ListBucketResult/>"),
            response(200, "<ListBucketResult/>"),
        ]);
        for prefix in [None, Some("folder/")] {
            let value = fixture::json(
                &fixture
                    .server()
                    .s3_list_objects(params(prefix))
                    .await
                    .unwrap(),
            );
            assert_eq!(value["count"], 0);
        }
        let requests = fixture.requests();
        assert!(!requests[0].headers.contains("prefix="));
        assert!(requests[1].headers.contains("prefix=folder%2F"));
        assert!(!requests[1].headers.contains("folder%2F%2F"));
    });
}

#[test]
fn truncated_pages_without_a_new_token_stop_without_looping() {
    fixture::run(async {
        let repeat = "<ListBucketResult><IsTruncated>true</IsTruncated><NextContinuationToken>same</NextContinuationToken></ListBucketResult>";
        let fixture = Fixture::new(vec![
            response(
                200,
                "<ListBucketResult><IsTruncated>true</IsTruncated></ListBucketResult>",
            ),
            response(200, repeat),
            response(200, repeat),
        ]);
        for _ in 0..2 {
            assert_eq!(
                fixture::json(
                    &fixture
                        .server()
                        .s3_list_objects(params(None))
                        .await
                        .unwrap()
                )["count"],
                0
            );
        }
        assert_eq!(fixture.requests().len(), 3);
    });
}

#[test]
fn continuing_pages_have_a_hard_request_limit() {
    fixture::run(async {
        let replies = (0..100).map(|number| response(200, format!("<ListBucketResult><IsTruncated>true</IsTruncated><NextContinuationToken>page{number}</NextContinuationToken></ListBucketResult>"))).collect();
        let fixture = Fixture::new(replies);
        let error = fixture
            .server()
            .s3_list_objects(params(None))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert!(error.message.contains("exceeded 100 pages"));
        assert_eq!(fixture.requests().len(), 100);
    });
}

#[test]
fn object_service_errors_are_propagated() {
    fixture::run(async {
        let fixture = Fixture::new(vec![response(
            403,
            "<Error><Code>AccessDenied</Code></Error>",
        )]);
        let error = fixture
            .server()
            .s3_list_objects(params(None))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(fixture.requests().len(), 1);
    });
}
