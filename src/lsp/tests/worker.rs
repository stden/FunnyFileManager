//! Purpose: Exercise the real threaded client against an isolated deterministic stdio peer.

use super::super::{
    client::Event,
    manager::{Command, Manager},
    positions::Position,
    types::{
        CompletionTrigger, DocumentId, Feature, Query, Revision, ServerState, Snapshot, Stamp,
    },
};
use crate::config::ServerSettings;
use std::{
    fs, thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

mod registration;

fn fixture(mode: &str) -> (TempDir, ServerSettings, Snapshot) {
    let root = TempDir::new().unwrap();
    let script = root.path().join("server.py");
    fs::write(&script, include_str!("server.py")).unwrap();
    let settings = ServerSettings {
        command: "python".into(),
        args: vec![script.to_string_lossy().into_owned(), mode.into()],
        ..ServerSettings::default()
    };
    let snapshot = Snapshot {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision(0),
        },
        path: root.path().join("main.rs"),
        text: "fn main() {}".into(),
        saved: 0,
    };
    (root, settings, snapshot)
}

fn send(manager: &Manager, command: Command) {
    assert!(manager.send(command).is_ok());
}

fn wait(manager: &Manager, predicate: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(event) = manager.poll() {
            if predicate(&event.1) {
                return event.1;
            }
            if matches!(event.1, Event::TrustRequired(_)) {
                send(manager, Command::Trust(event.0)); // The test owns this temporary project.
            }
        }
        assert!(Instant::now() < deadline, "LSP event timed out");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn untrusted_workspaces_never_start_a_child_and_stale_trust_cannot_approve_them() {
    let (root, settings, snapshot) = fixture("normal");
    let manager = Manager::start().unwrap();
    send(&manager, Command::Open("rust".into(), settings, snapshot));
    let Event::TrustRequired(path) =
        wait(&manager, |event| matches!(event, Event::TrustRequired(_)))
    else {
        panic!()
    };
    assert_eq!(path, root.path().canonicalize().unwrap());
    send(&manager, Command::Trust(DocumentId::default()));
    drop(manager);
    assert!(!root.path().join("methods").exists());
    assert!(!root.path().join("pid").exists());
}

#[test]
fn worker_initializes_synchronizes_answers_and_shuts_down_in_order() {
    let (root, settings, mut snapshot) = fixture("normal");
    let manager = Manager::start().unwrap();
    send(
        &manager,
        Command::Open("rust".into(), settings, snapshot.clone()),
    );
    wait(&manager, |event| matches!(event, Event::Diagnostics(_, _)));
    snapshot.stamp.revision = Revision(1);
    snapshot.text.push('\n');
    send(&manager, Command::Update(snapshot.clone()));
    wait(
        &manager,
        |event| matches!(event, Event::Diagnostics(stamp, _) if stamp.revision == Revision(1)),
    );
    send(
        &manager,
        Command::Query(Query {
            stamp: snapshot.stamp,
            position: Position::default(),
            feature: Feature::Hover,
        }),
    );
    wait(&manager, |event| matches!(event, Event::Reply(_, Ok(_))));
    drop(manager);
    let methods = fs::read_to_string(root.path().join("methods")).unwrap();
    assert!(methods.starts_with("initialize\ninitialized\ntextDocument/didOpen\n"));
    assert!(methods.contains("textDocument/didChange\n"));
    assert!(methods.ends_with("textDocument/didClose\nshutdown\nexit\n"));
}

#[test]
fn missing_command_and_broken_servers_report_states_without_panicking() {
    for mode in ["missing", "crash", "malformed"] {
        let (_root, mut settings, snapshot) = fixture(mode);
        if mode == "missing" {
            settings.command = snapshot
                .path
                .with_extension("absent")
                .to_string_lossy()
                .into_owned();
        }
        let manager = Manager::start().unwrap();
        send(&manager, Command::Open("rust".into(), settings, snapshot));
        let expected = if mode == "missing" {
            ServerState::Unavailable
        } else {
            ServerState::Failed
        };
        wait(
            &manager,
            |event| matches!(event, Event::State(state) if state == &expected),
        );
    }
}

#[test]
fn uncooperative_server_shutdown_is_bounded() {
    let (_root, settings, snapshot) = fixture("hang");
    let manager = Manager::start().unwrap();
    send(&manager, Command::Open("rust".into(), settings, snapshot));
    wait(&manager, |event| {
        matches!(event, Event::State(ServerState::Ready))
    });
    let started = Instant::now();
    drop(manager);
    assert!(started.elapsed() < Duration::from_secs(4));
}

#[test]
fn a_crash_during_shutdown_is_reaped_without_waiting_for_a_response() {
    let (_root, settings, snapshot) = fixture("shutdown-crash");
    let manager = Manager::start().unwrap();
    send(&manager, Command::Open("rust".into(), settings, snapshot));
    wait(&manager, |event| {
        matches!(event, Event::State(ServerState::Ready))
    });
    let started = Instant::now();
    drop(manager);
    assert!(started.elapsed() < Duration::from_secs(4));
}
#[test]
fn unsupported_server_features_return_errors_without_stopping_the_session() {
    let (_root, settings, snapshot) = fixture("unsupported");
    let manager = Manager::start().unwrap();
    send(
        &manager,
        Command::Open("rust".into(), settings, snapshot.clone()),
    );
    wait(&manager, |event| {
        matches!(event, Event::State(ServerState::Ready))
    });
    send(
        &manager,
        Command::Query(Query {
            stamp: snapshot.stamp,
            position: Position::default(),
            feature: Feature::Hover,
        }),
    );
    let event = wait(&manager, |event| matches!(event, Event::Reply(_, _)));
    assert!(matches!(event, Event::Reply(_, Err(error)) if error.code == -32601));
    send(
        &manager,
        Command::Query(Query {
            stamp: snapshot.stamp,
            position: Position::default(),
            feature: Feature::Completion(CompletionTrigger::Manual),
        }),
    );
    wait(&manager, |event| matches!(event, Event::Reply(_, Ok(_))));
}
