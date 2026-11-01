//! Purpose: Exercise the private child protocol without processes or environment changes.

use super::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod resources;
mod wire;

struct FakeLauncher {
    handshake: &'static str,
    reply: &'static str,
}

impl launch::Launcher for FakeLauncher {
    fn launch(
        &self,
        _: &std::path::Path,
        _: Size,
        _: Profile,
    ) -> Result<connection::Connection, Error> {
        let (client, server) = tokio::io::duplex(4096);
        let (read, write) = tokio::io::split(client);
        let handshake = self.handshake;
        let reply = self.reply;
        tokio::spawn(fake_peer(server, handshake, reply));
        Ok(connection::Connection::from_io(read, write))
    }
}

async fn fake_peer(stream: tokio::io::DuplexStream, handshake: &str, reply: &str) {
    let (reader, mut writer) = tokio::io::split(stream);
    writer.write_all(handshake.as_bytes()).await.unwrap();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap() != 0 {
        assert!(serde_json::from_str::<Value>(&line).unwrap()["operation"].is_string());
        if writer.write_all(reply.as_bytes()).await.is_err() {
            break;
        }
        line.clear();
    }
}

fn manager(handshake: &'static str, reply: &'static str) -> SessionManager {
    SessionManager::with_launcher(Arc::new(FakeLauncher { handshake, reply }))
}

#[tokio::test]
async fn handshake_and_state_exchange_use_private_json_frames() {
    let manager = manager(
        "{\"ok\":true,\"result\":{\"protocol_version\":1,\"state\":{\"ready\":true}}}\n",
        "{\"ok\":true,\"result\":{\"active\":\"left\"}}\n",
    );
    let root = tempfile::tempdir().unwrap();
    let started = manager
        .start(root.path(), Size::new(120, 40).unwrap(), Profile::System)
        .await
        .unwrap();
    assert_eq!(started.state, json!({"ready":true}));
    assert_eq!(
        manager
            .request(&started.session_id, Request::State)
            .await
            .unwrap(),
        json!({"active":"left"})
    );
}

#[tokio::test]
async fn incompatible_handshake_is_refused_without_a_session() {
    let manager = manager(
        "{\"ok\":true,\"result\":{\"protocol_version\":2,\"state\":{}}}\n",
        "",
    );
    let root = tempfile::tempdir().unwrap();
    assert!(manager
        .start(root.path(), Size::new(80, 24).unwrap(), Profile::System)
        .await
        .is_err());
    assert!(manager.sessions.lock().await.is_empty());
    drop(manager);
}

#[tokio::test]
async fn application_error_does_not_desynchronize_the_session() {
    let manager = manager(
        "{\"ok\":true,\"result\":{\"protocol_version\":1,\"state\":{}}}\n",
        "{\"ok\":false,\"error\":{\"code\":\"busy\",\"message\":\"job running\"}}\n",
    );
    let root = tempfile::tempdir().unwrap();
    let started = manager
        .start(root.path(), Size::new(80, 24).unwrap(), Profile::System)
        .await
        .unwrap();
    let error = manager
        .request(&started.session_id, Request::State)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("job running"));
    assert!(manager
        .request(&started.session_id, Request::State)
        .await
        .unwrap_err()
        .to_string()
        .contains("job running"));
    assert_eq!(manager.sessions.lock().await.len(), 1);
    drop(manager);
}

#[tokio::test]
async fn malformed_response_removes_the_broken_session() {
    let manager = manager(
        "{\"ok\":true,\"result\":{\"protocol_version\":1,\"state\":{}}}\n",
        "terminal garbage\n",
    );
    let root = tempfile::tempdir().unwrap();
    let started = manager
        .start(root.path(), Size::new(80, 24).unwrap(), Profile::System)
        .await
        .unwrap();
    assert!(manager
        .request(&started.session_id, Request::State)
        .await
        .is_err());
    assert!(manager.sessions.lock().await.is_empty());
    drop(manager);
}

#[test]
fn dimensions_refuse_unbounded_terminal_allocations() {
    assert!(Size::new(0, 40).is_err());
    assert!(Size::new(120, 0).is_err());
    assert!(Size::new(20, 8).is_ok());
    assert!(Size::new(19, 8).is_err());
    assert!(Size::new(20, 7).is_err());
    assert!(Size::new(400, 200).is_ok());
    assert!(Size::new(401, 200).is_err());
    assert!(Size::new(120, 201).is_err());
}
