use super::*;

#[test]
fn deletion_and_copy_send_the_requested_object_locations() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            response(204, ""),
            response(
                200,
                "<CopyObjectResult><ETag>fixture</ETag></CopyObjectResult>",
            ),
        ]);
        let server = fixture.server();
        let value = fixture::json(&server.s3_delete(delete()).await.unwrap());
        assert_eq!(value["bucket"], "bucket");
        assert_eq!(value["deleted"], "folder/object");
        let value = fixture::json(&server.s3_copy(copy()).await.unwrap());
        drop(server);
        assert_eq!(value["source"], "s3://source/folder/object");
        assert_eq!(value["destination"], "s3://destination/copy");
        let requests = fixture.requests();
        assert!(requests[0].headers.starts_with("DELETE "));
        assert_eq!(requests[0].path(), "/bucket/folder/object");
        assert!(requests[1].headers.starts_with("PUT "));
        assert_eq!(requests[1].path(), "/destination/copy");
        assert!(requests[1]
            .headers
            .to_ascii_lowercase()
            .contains("x-amz-copy-source: source/folder/object"));
    });
}

#[test]
fn deletion_and_copy_errors_are_not_reported_as_successes() {
    fixture::run(async {
        let fixture = Fixture::new(vec![denied(), denied()]);
        let server = fixture.server();
        let errors = [
            server.s3_delete(delete()).await.unwrap_err(),
            server.s3_copy(copy()).await.unwrap_err(),
        ];
        drop(server);
        for error in errors {
            assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        }
        assert_eq!(fixture.requests().len(), 2);
    });
}

#[test]
fn readonly_blocks_all_remote_mutations_before_any_connection() {
    fixture::run(async {
        let fixture = Fixture::new(Vec::new());
        let server = FfmServer::with_config(fixture.root.path().to_path_buf(), true, false);
        let results = [
            server.s3_download(transfer("file")).await,
            server.s3_upload(transfer("file")).await,
            server.s3_delete(delete()).await,
            server.s3_copy(copy()).await,
        ];
        drop(server);
        for result in results {
            let error = result.unwrap_err();
            assert!(error.message.contains("read-only"));
        }
        assert!(fixture.requests().is_empty());
    });
}

#[test]
fn invalid_profiles_cannot_reach_any_remote_mutation() {
    fixture::run(async {
        let fixture = Fixture::new(Vec::new());
        let server = fixture.server();
        let mut download = transfer("file");
        download.0.profile = "invalid/profile".into();
        let mut upload = transfer("file");
        upload.0.profile = "invalid/profile".into();
        let mut delete = delete();
        delete.0.profile = "invalid/profile".into();
        let mut copy = copy();
        copy.0.profile = "invalid/profile".into();
        let results = [
            server.s3_download(download).await,
            server.s3_upload(upload).await,
            server.s3_delete(delete).await,
            server.s3_copy(copy).await,
        ];
        drop(server);
        for result in results {
            assert_eq!(result.unwrap_err().code, ErrorCode::INVALID_PARAMS);
        }
        assert!(fixture.requests().is_empty());
    });
}
