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
    assert_eq!(messages.len(), 0);
    drop(receiver);
    messages.push(json!(4));
    assert!(flush_messages(&mut messages, |message| sender.try_send(message)).is_err());
}

#[test]
fn an_unresponsive_writer_cannot_grow_the_outbox_without_bound() {
    let mut messages = vec![json!(0); 1025];
    assert!(flush_messages(&mut messages, |_| panic!("must reject before sending")).is_err());
}

fn silent_connection(root: &Path) -> Connection {
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
        Err(PumpError::Disconnected("LSP initialization timed out"))
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

#[test]
fn pre_ready_crash_without_hint_reports_disconnected() {
    let root = tempfile::TempDir::new().unwrap();
    let crash_cfg = ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "raise SystemExit(1)".into()],
        ..ServerSettings::default()
    };
    let snap = silent_connection(root.path()).pending.clone().unwrap();
    let mut crashed = Connection::start("rust".into(), &crash_cfg, snap, root.path()).unwrap();
    let (events, _rx) = mpsc::sync_channel(8);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Err(err) = crashed.pump(&events) {
            assert_eq!(err, PumpError::Disconnected("LSP disconnected"));
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TICK);
    }
}

#[test]
fn broken_shutdown_flushes_terminate_cleanly() {
    let root = tempfile::TempDir::new().unwrap();
    let init = json!({"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}});
    let mut fail_first = silent_connection(root.path());
    fail_first.client.receive(init.clone()).unwrap();
    fail_first.process.set_outgoing(None);
    drop(fail_first);

    let mut fail_exit = silent_connection(root.path());
    fail_exit.client.receive(init.clone()).unwrap();
    fail_exit.client.shutdown_complete = true;
    let (full_tx, _full_rx) = mpsc::sync_channel(0);
    fail_exit.process.set_outgoing(Some(full_tx));
    fail_exit.client.rpc.outbox.resize(1023, json!(0));
    drop(fail_exit);

    let mut fail_loop = silent_connection(root.path());
    fail_loop.client.receive(init).unwrap();
    fail_loop.client.shutdown_complete = true;
    let (tx, rx) = mpsc::sync_channel(1);
    tx.send(json!(0)).unwrap();
    fail_loop.process.set_outgoing(Some(tx));
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5));
        drop(rx);
    });
    drop(fail_loop);
}
