use super::*;

fn params(paths: &[&str], destination: Option<&str>) -> Parameters<BatchParams> {
    Parameters(BatchParams { paths: paths.iter().map(ToString::to_string).collect(), destination: destination.map(str::to_owned), recursive: true })
}

#[test]
fn empty_batches_create_the_destination_and_missing_destination_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    server.batch_copy(params(&[], Some("copied"))).unwrap();
    server.batch_move(params(&[], Some("moved"))).unwrap();
    assert!(root.path().join("copied").is_dir());
    assert!(root.path().join("moved").is_dir());
    assert_eq!(server.batch_copy(params(&[], None)).unwrap_err().message, "destination is required");
    drop(server);
}

#[test]
fn batch_mutations_refuse_the_server_root_and_keep_it_intact() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let deleted = server.batch_delete(params(&["."], None)).unwrap();
    let moved = server.batch_move(params(&["."], Some("elsewhere"))).unwrap();
    for result in [&deleted, &moved] {
        let value = crate::tests::tool_json(result);
        assert_eq!(value["total_errors"], 1);
        assert_eq!(value["errors"][0]["error"], "cannot delete or move root directory");
    }
    assert!(root.path().is_dir());
    drop(server);
}
