//! Purpose: Preserve ordered protocol messages under bounded writer backpressure.

use super::*;
use serde_json::json;
use std::sync::mpsc;

#[test]
fn a_full_writer_retries_messages_in_order_without_losing_them() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let mut messages = vec![json!(1), json!(2), json!(3)];
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(1));
    assert_eq!(messages, [json!(2), json!(3)]);
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(2));
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(3));
    assert!(messages.is_empty());
    drop(receiver);
    messages.push(json!(4));
    assert!(flush_messages(&mut messages, |message| sender.try_send(message)).is_err());
}

#[test]
fn an_unresponsive_writer_cannot_grow_the_outbox_without_bound() {
    let mut messages = vec![json!(0); 1025];
    assert!(flush_messages(&mut messages, |_| panic!("must reject before sending")).is_err());
}

fn silent_connection(root: &std::path::Path) -> Connection {
    let settings = ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "import time; time.sleep(30)".into()],
        ..ServerSettings::default()
    };
    let snapshot = Snapshot {
        stamp: super::super::types::Stamp {
            id: DocumentId::default(),
            revision: crate::lsp::types::Revision::default(),
        },
        path: root.join("main.rs"),
        text: "source".into(),
        saved: 0,
    };
    Connection::start("rust".into(), &settings, snapshot, root).unwrap()
}

#[test]
fn initialization_times_out_without_waiting_for_a_silent_process() {
    let root = tempfile::TempDir::new().unwrap();
    let mut connection = silent_connection(root.path());
    let mut unrelated = connection.pending.clone().unwrap();
    unrelated.stamp.id = DocumentId::default();
    unrelated.text = "another document".into();
    connection.command(Command::Update(unrelated));
    assert_eq!(connection.pending.as_ref().unwrap().text, "source");
    connection.started = Instant::now()
        .checked_sub(INITIALIZE_TIMEOUT + TICK)
        .unwrap();
    let (sender, _receiver) = mpsc::sync_channel(8);
    assert_eq!(
        connection.pump(&sender),
        Err("LSP initialization timed out")
    );
    let started = Instant::now();
    drop(connection);
    assert!(started.elapsed() < Duration::from_secs(4));
}

#[test]
fn edits_before_initialization_preserve_the_open_document_path() {
    let root = tempfile::TempDir::new().unwrap();
    let mut connection = silent_connection(root.path());
    let mut snapshot = connection.pending.clone().unwrap();
    let original = snapshot.path.clone();
    snapshot.path = root.path().join("alias.rs");
    snapshot.text = "updated".into();
    connection.command(Command::Update(snapshot));
    let pending = connection.pending.as_ref().unwrap();
    assert_eq!(pending.path, original);
    assert_eq!(pending.text, "updated");
}
