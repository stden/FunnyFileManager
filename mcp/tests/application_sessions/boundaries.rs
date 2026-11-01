//! Purpose: Real MCP guard errors precede child launch and file tools retain confinement.

use super::client::{Access, Client};
use serde_json::json;
use std::fs;

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn disabled_and_readonly_servers_refuse_sessions_before_launching_ffm() {
    let root = tempfile::tempdir().unwrap();
    for (access, expected) in [
        (Access::Disabled, "FFM_MCP_ALLOW_APPLICATION=1"),
        (Access::Readonly, "read-only"),
    ] {
        let mut client = Client::spawn(root.path(), access).await;
        let error = client
            .result("ffm_session_start", json!({"profile":"isolated"}))
            .await
            .unwrap_err();
        assert!(error.contains(expected), "{error}");
        for (tool, arguments) in [
            ("ffm_session_state", json!({"session_id":"absent"})),
            (
                "ffm_session_input",
                json!({"session_id":"absent","text":"do not run"}),
            ),
        ] {
            let error = client.result(tool, arguments).await.unwrap_err();
            assert!(error.contains(expected), "{error}");
        }
        client.close().await;
    }
}

#[tokio::test]
#[ignore = "requires FFM_APPLICATION_TEST_BINARY pointing to a separately built ffm executable"]
async fn file_tools_remain_root_confined_when_full_application_sessions_are_enabled() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let kept = outside.path().join("kept.txt");
    fs::write(&kept, "outside fixture").unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let id = client.start(root.path()).await;
    for tool in ["view_file", "delete"] {
        let error = client
            .result(tool, json!({"path":kept.to_string_lossy()}))
            .await
            .unwrap_err();
        assert!(error.contains("outside"), "{tool}: {error}");
    }
    assert_eq!(fs::read(&kept).unwrap(), b"outside fixture");
    client.close_session(&id).await;
    client.close().await;
}
