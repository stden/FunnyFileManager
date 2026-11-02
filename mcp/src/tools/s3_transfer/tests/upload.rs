use super::*;

#[test]
fn uploading_sends_the_local_bytes_and_reports_their_size() {
    fixture::run(async {
        let fixture = Fixture::new(vec![response(200, "")]);
        let bytes = b"uploaded binary\0bytes";
        fs::write(fixture.root.path().join("upload"), bytes).unwrap();
        let value = fixture::json(
            &fixture
                .server()
                .s3_upload(transfer("upload"))
                .await
                .unwrap(),
        );
        assert_eq!(value["size"], bytes.len());
        assert_eq!(value["bucket"], "bucket");
        let request = &fixture.requests()[0];
        assert!(request.headers.starts_with("PUT "));
        assert_eq!(request.path(), "/bucket/folder/object");
        assert!(request
            .body
            .windows(bytes.len())
            .any(|window| window == bytes));
    });
}

#[test]
fn oversized_local_files_are_refused_without_uploading() {
    fixture::run(async {
        let fixture = Fixture::new(Vec::new());
        let path = fixture.root.path().join("large");
        fs::File::create(&path)
            .unwrap()
            .set_len(MAX_S3_OBJECT_BYTES + 1)
            .unwrap();
        let error = fixture
            .server()
            .s3_upload(transfer("large"))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert_eq!(error.message, "local file too large");
        assert!(fixture.requests().is_empty());
    });
}

#[test]
fn missing_and_outside_files_are_refused_without_uploading() {
    fixture::run(async {
        let fixture = Fixture::new(Vec::new());
        let outside = tempfile::NamedTempFile::new().unwrap();
        for path in ["missing".to_owned(), outside.path().display().to_string()] {
            let error = fixture
                .server()
                .s3_upload(transfer(&path))
                .await
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        }
        assert!(fixture.requests().is_empty());
    });
}

#[test]
fn upload_service_errors_preserve_the_source() {
    fixture::run(async {
        let fixture = Fixture::new(vec![denied()]);
        let path = fixture.root.path().join("upload");
        fs::write(&path, b"original").unwrap();
        let error = fixture
            .server()
            .s3_upload(transfer("upload"))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(fs::read(path).unwrap(), b"original");
    });
}
