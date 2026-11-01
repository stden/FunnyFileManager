//! Purpose: Prove application authority cannot bypass opt-in or read-only guards.

use crate::FfmServer;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

macro_rules! refuses {
    ($server:ident, $method:ident, $arguments:expr, $message:ident) => {
        assert!($server
            .$method(Parameters(serde_json::from_value($arguments).unwrap()))
            .await
            .unwrap_err()
            .message
            .contains($message));
    };
}

async fn all_tools_refuse(server: &FfmServer, message: &str) {
    let session = json!({"session_id":"untrusted"});
    refuses!(server, ffm_session_start, json!({}), message);
    refuses!(server, ffm_session_catalog, session.clone(), message);
    refuses!(server, ffm_session_state, session.clone(), message);
    mutation_tools_refuse(server, message).await;
    refuses!(server, ffm_session_cancel, session.clone(), message);
    refuses!(server, ffm_session_close, session, message);
}

async fn mutation_tools_refuse(server: &FfmServer, message: &str) {
    refuses!(
        server,
        ffm_session_action,
        json!({"session_id":"untrusted","context":"panels","action":"quit"}),
        message
    );
    refuses!(
        server,
        ffm_session_input,
        json!({"session_id":"untrusted","text":"secret"}),
        message
    );
    refuses!(
        server,
        ffm_session_wait,
        json!({"session_id":"untrusted","timeout_ms":10001}),
        message
    );
    refuses!(
        server,
        ffm_session_resize,
        json!({"session_id":"untrusted","width":0,"height":0}),
        message
    );
}

#[tokio::test]
async fn every_application_tool_requires_explicit_opt_in() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().to_path_buf(), false, false);
    all_tools_refuse(&server, "FFM_MCP_ALLOW_APPLICATION=1").await;
}

#[tokio::test]
async fn readonly_wins_even_when_application_flag_is_enabled() {
    let root = tempfile::tempdir().unwrap();
    let mut server = FfmServer::with_config(root.path().to_path_buf(), true, false);
    server.application_allowed = true;
    all_tools_refuse(&server, "read-only mode").await;
    drop(server);
}

#[tokio::test]
async fn initial_path_must_still_be_inside_the_file_tool_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    server.application_allowed = true;
    let error = server
        .ffm_session_start(Parameters(
            serde_json::from_value(json!({"path":outside.path().display().to_string()})).unwrap(),
        ))
        .await
        .unwrap_err();
    drop(server);
    assert!(error.message.contains("outside"));
}

#[tokio::test]
async fn server_clones_share_the_same_session_registry() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().to_path_buf(), false, false);
    assert!(std::sync::Arc::ptr_eq(
        &server.application,
        &server.clone().application
    ));
}

#[tokio::test]
async fn oversized_wait_and_input_are_refused_before_transport() {
    let root = tempfile::tempdir().unwrap();
    let mut server = FfmServer::with_config(root.path().to_path_buf(), false, false);
    server.application_allowed = true;
    let message = "at most 10000";
    refuses!(
        server,
        ffm_session_wait,
        json!({"session_id":"unused","timeout_ms":10001}),
        message
    );
    let message = "128 key chords";
    refuses!(
        server,
        ffm_session_input,
        json!({"session_id":"unused","keys":vec!["F1";129]}),
        message
    );
    drop(server);
}
