//! Purpose: Exercise enabled FFM application tools end-to-end over in-memory pipes.

use super::super::helpers::automation_error;
use crate::{
    automation::{connection::Connection, launch::Launcher, Error, Profile, SessionManager, Size},
    FfmServer,
};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ErrorCode},
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

struct EchoLauncher;

impl Launcher for EchoLauncher {
    fn launch(&self, _: &Path, _: Size, _: Profile) -> Result<Connection, Error> {
        let (client, server) = tokio::io::duplex(8192);
        let (read, write) = tokio::io::split(client);
        tokio::spawn(echo_peer(server));
        Ok(Connection::from_io(read, write))
    }
}

async fn echo_peer(stream: DuplexStream) {
    let (reader, mut writer) = tokio::io::split(stream);
    let ready = "{\"ok\":true,\"result\":{\"protocol_version\":1,\"state\":{\"ready\":true}}}\n";
    if writer.write_all(ready.as_bytes()).await.is_err() {
        return;
    }
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let parsed: Value = serde_json::from_str(&line).unwrap();
        let reply = format!("{{\"ok\":true,\"result\":{{\"echo\":{parsed}}}}}\n");
        if writer.write_all(reply.as_bytes()).await.is_err() {
            break;
        }
    }
}

fn enabled_server(root: &Path) -> FfmServer {
    let mut server = FfmServer::with_config(root.canonicalize().unwrap(), false, false);
    server.application_allowed = true;
    server.application = Arc::new(SessionManager::with_launcher(Arc::new(EchoLauncher)));
    server
}

fn args<T: DeserializeOwned>(value: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(value).unwrap())
}

fn payload(result: &CallToolResult) -> Value {
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}

async fn check_queries_and_actions(server: &FfmServer, session_id: &str) {
    let id = json!({"session_id": session_id});
    let catalog = server.ffm_session_catalog(args(id.clone())).await.unwrap();
    assert_eq!(payload(&catalog)["echo"]["operation"], "catalog");

    let state = server.ffm_session_state(args(id)).await.unwrap();
    assert_eq!(payload(&state)["echo"]["operation"], "state");

    let act = json!({"session_id": session_id, "context": "panels", "action": "quit"});
    let action = server.ffm_session_action(args(act)).await.unwrap();
    assert_eq!(payload(&action)["echo"]["operation"], "action");
}

async fn check_input_resize_wait_cancel(server: &FfmServer, session_id: &str) {
    let inp = json!({"session_id": session_id, "keys": ["Down"], "text": "hi"});
    let input = server.ffm_session_input(args(inp)).await.unwrap();
    assert_eq!(payload(&input)["echo"]["operation"], "input");

    let res = json!({"session_id": session_id, "width": 100, "height": 30});
    let resize = server.ffm_session_resize(args(res)).await.unwrap();
    assert_eq!(payload(&resize)["echo"]["width"], 100);

    let wt = json!({"session_id": session_id, "timeout_ms": 250});
    let wait = server.ffm_session_wait(args(wt)).await.unwrap();
    assert_eq!(payload(&wait)["echo"]["timeout_ms"], 250);

    let id = json!({"session_id": session_id});
    let cancel = server.ffm_session_cancel(args(id)).await.unwrap();
    assert_eq!(payload(&cancel)["echo"]["operation"], "cancel");
}

#[tokio::test]
async fn full_session_lifecycle_dispatches_every_tool_over_private_pipes() {
    let root = tempfile::tempdir().unwrap();
    let server = enabled_server(root.path());
    let started = server.ffm_session_start(args(json!({}))).await.unwrap();
    let started_json = payload(&started);
    let session_id = started_json["session_id"].as_str().unwrap().to_owned();
    assert_eq!(started_json["protocol_version"], 1);
    assert_eq!(started_json["state"]["ready"], true);

    check_queries_and_actions(&server, &session_id).await;
    check_input_resize_wait_cancel(&server, &session_id).await;

    let id = json!({"session_id": &session_id});
    let close = server.ffm_session_close(args(id.clone())).await.unwrap();
    assert_eq!(payload(&close)["echo"]["operation"], "close");

    let err = server.ffm_session_state(args(id)).await.unwrap_err();
    drop(server);
    assert!(err.message.contains("unknown or closed"));
}

#[tokio::test]
async fn start_input_and_resize_validate_paths_bounds_and_payload_limits() {
    let root = tempfile::tempdir().unwrap();
    let sub = root.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let file = root.path().join("plain.txt");
    std::fs::write(&file, "x").unwrap();
    let server = enabled_server(root.path());

    let start_ok = json!({"path": sub.display().to_string(), "profile": "isolated"});
    assert!(server.ffm_session_start(args(start_ok)).await.is_ok());

    let start_file = json!({"path": file.display().to_string()});
    assert!(server.ffm_session_start(args(start_file)).await.is_err());

    let bad_key = json!({"session_id": "x", "keys": ["a".repeat(129)]});
    let key_err = server.ffm_session_input(args(bad_key)).await.unwrap_err();
    assert!(key_err.message.contains("128 key chords"));

    let bad_dim = json!({"session_id": "x", "width": 19, "height": 24});
    let dim_err = server.ffm_session_resize(args(bad_dim)).await.unwrap_err();
    drop(server);
    assert!(dim_err.message.contains("width"));
}

#[tokio::test]
async fn input_rejects_text_over_one_mib_and_maps_all_automation_errors() {
    let root = tempfile::tempdir().unwrap();
    let bad_text = json!({"session_id": "x", "text": "x".repeat(1024 * 1024 + 1)});
    let err = enabled_server(root.path())
        .ffm_session_input(args(bad_text))
        .await
        .unwrap_err();
    assert!(err.message.contains("1 MiB"));
    assert_automation_error_codes();
}

fn assert_automation_error_codes() {
    for client_err in [
        Error::Invalid("bad"),
        Error::UnknownSession,
        Error::SessionLimit,
        Error::Application {
            code: "busy".to_owned(),
            message: "wait".to_owned(),
        },
    ] {
        assert_eq!(automation_error(client_err).code, ErrorCode::INVALID_PARAMS);
    }
    for internal_err in [
        Error::Io(std::io::Error::other("pipe")),
        Error::Protocol("eof"),
        Error::Timeout,
    ] {
        assert_eq!(
            automation_error(internal_err).code,
            ErrorCode::INTERNAL_ERROR
        );
    }
}
