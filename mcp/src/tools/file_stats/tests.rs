use super::*;

#[test]
fn oversized_text_is_refused_before_it_is_streamed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("large");
    fs::File::create(&path)
        .unwrap()
        .set_len(100 * 1024 * 1024 + 1)
        .unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let error = server
        .file_stats(Parameters(PathParam {
            path: "large".into(),
        }))
        .unwrap_err();
    drop(server);
    assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    assert!(error.message.contains("maximum size"));
}

#[test]
fn a_file_disappearing_before_text_counting_returns_an_internal_error() {
    let root = tempfile::tempdir().unwrap();
    let error = count_file_text(&root.path().join("absent")).err().unwrap();
    assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
}
