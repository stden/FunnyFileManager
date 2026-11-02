use super::*;

#[test]
fn downloading_creates_parent_directories_and_replaces_existing_bytes() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            response(200, b"first\0payload"),
            response(200, b"replacement"),
        ]);
        let path = fixture.root.path().join("nested/download.bin");
        for expected in [b"first\0payload".as_slice(), b"replacement".as_slice()] {
            let value = fixture::json(
                &fixture
                    .server()
                    .s3_download(transfer("nested/download.bin"))
                    .await
                    .unwrap(),
            );
            assert_eq!(fs::read(&path).unwrap(), expected);
            assert_eq!(value["size"], expected.len());
            assert_eq!(value["key"], "folder/object");
            assert_eq!(value["local_path"], path.display().to_string());
        }
        let requests = fixture.requests();
        assert!(requests[0].headers.starts_with("GET "));
        assert_eq!(requests[0].path(), "/bucket/folder/object");
    });
}

#[test]
fn unknown_lengths_are_checked_after_the_body_is_collected() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            b"HTTP/1.1 200 Fixture\r\nConnection: close\r\n\r\nno length".to_vec(),
        ]);
        let value = fixture::json(
            &fixture
                .server()
                .s3_download(transfer("download"))
                .await
                .unwrap(),
        );
        assert_eq!(value["size"], 9);
        assert_eq!(
            fs::read(fixture.root.path().join("download")).unwrap(),
            b"no length"
        );
    });
}

#[test]
fn advertised_oversized_downloads_never_create_a_destination() {
    fixture::run(async {
        let reply = format!(
            "HTTP/1.1 200 Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_S3_DOWNLOAD_BYTES + 1
        )
        .into_bytes();
        let fixture = Fixture::new(vec![reply]);
        let error = fixture
            .server()
            .s3_download(transfer("download"))
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert_eq!(error.message, "Object too large");
        assert!(!fixture.root.path().join("download").exists());
    });
}

#[test]
fn service_and_incomplete_body_errors_leave_local_files_unchanged() {
    fixture::run(async {
        let fixture = Fixture::new(vec![
            denied(),
            b"HTTP/1.1 200 Fixture\r\nContent-Length: 20\r\nConnection: close\r\n\r\nshort"
                .to_vec(),
        ]);
        let path = fixture.root.path().join("download");
        fs::write(&path, b"original").unwrap();
        for _ in 0..2 {
            let error = fixture
                .server()
                .s3_download(transfer("download"))
                .await
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
            assert_eq!(fs::read(&path).unwrap(), b"original");
        }
    });
}

#[test]
fn writing_a_download_reports_real_destination_errors() {
    let fixture = tempfile::tempdir().unwrap();
    let blocker = fixture.path().join("file");
    fs::write(&blocker, b"unchanged").unwrap();
    assert!(write_downloaded(&blocker.join("child"), b"new").is_err());
    assert!(write_downloaded(fixture.path(), b"new").is_err());
    assert!(write_downloaded(std::path::Path::new(""), b"new").is_err());
    assert_eq!(fs::read(blocker).unwrap(), b"unchanged");
}
