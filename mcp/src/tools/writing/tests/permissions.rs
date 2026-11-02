use super::*;
use std::os::unix::fs::PermissionsExt;

fn results(server: &FfmServer, path: &str) -> [Result<CallToolResult, McpError>; 3] {
    [
        server.create_file(Parameters(CreateFileParams {
            path: path.into(),
            content: None,
        })),
        server.write_file(Parameters(WriteFileParams {
            path: path.into(),
            content: "x".into(),
        })),
        server.append_file(Parameters(AppendFileParams {
            path: path.into(),
            content: "x".into(),
        })),
    ]
}

#[test]
fn inaccessible_parent_errors_reach_each_writing_tool() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        let server = server(&root);
        let directory = root.path().join("locked");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o555)).unwrap();
        let errors: Vec<_> = ["locked/new/file", "locked/file"]
            .into_iter()
            .flat_map(|path| results(&server, path))
            .collect();
        drop(server);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        for result in errors {
            assert_eq!(
                result.unwrap_err().code,
                rmcp::model::ErrorCode::INTERNAL_ERROR
            );
        }
        assert!(fs::read_dir(directory).unwrap().next().is_none());
    });
}

#[test]
fn appending_to_a_readonly_file_preserves_its_bytes() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("readonly");
        fs::write(&path, "original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        let error = server(&root)
            .append_file(Parameters(AppendFileParams {
                path: "readonly".into(),
                content: "new".into(),
            }))
            .unwrap_err();
        assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
    });
}
