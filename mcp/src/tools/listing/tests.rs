use super::*;

#[test]
fn directory_information_identifies_the_directory_type() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let result = server.get_file_info(Parameters(PathParam { path: ".".into() })).unwrap();
    assert_eq!(crate::tests::tool_json(&result)["type"], "directory");
    drop(server);
}
