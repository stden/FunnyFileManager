//! Purpose: Verify workspace trust isolation and launch failures without executing project code.

use super::*;
use std::path::Path;
use std::sync::mpsc;
use tempfile::TempDir;

fn snapshot(root: &Path) -> Snapshot {
    Snapshot {
        stamp: crate::lsp::types::Stamp {
            id: DocumentId::default(),
            revision: crate::lsp::types::Revision::default(),
        },
        path: root.join("main.rs"),
        text: "original".into(),
        saved: 0,
    }
}

#[test]
fn pending_trust_retains_edits_and_ignores_approval_for_another_document() {
    let root = TempDir::new().unwrap();
    let mut snapshot = snapshot(root.path());
    let (events, received) = mpsc::sync_channel(8);
    let mut host = Host::default();
    host.command(
        Command::Open("rust".into(), ServerSettings::default(), snapshot.clone()),
        &events,
    );
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::TrustRequired(_)
    ));
    host.command(Command::Trust(DocumentId::default()), &events);
    host.command(Command::Cancel, &events);
    snapshot.text = "edited before approval".into();
    host.command(Command::Update(snapshot.clone()), &events);
    let Session::Untrusted(start) = &host.session else {
        panic!("must remain untrusted")
    };
    assert_eq!(start.snapshot.text, snapshot.text);
    assert!(
        received.try_recv().is_err(),
        "no server may start from stale approval"
    );
    host.command(Command::Close, &events);
    assert!(matches!(host.session, Session::Closed));
    host.command(Command::Update(snapshot), &events);
    assert!(matches!(host.session, Session::Closed));
}

#[test]
fn an_unavailable_root_does_not_start_a_server() {
    let root = TempDir::new().unwrap();
    let (events, received) = mpsc::sync_channel(8);
    let mut host = Host::default();
    host.command(
        Command::Open(
            "rust".into(),
            ServerSettings::default(),
            snapshot(&root.path().join("absent")),
        ),
        &events,
    );
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Unavailable)
    ));
    assert!(matches!(host.session, Session::Closed));
}

#[test]
fn an_invalid_executable_fails_after_explicit_workspace_approval() {
    let root = TempDir::new().unwrap();
    let (events, received) = mpsc::sync_channel(8);
    let mut host = Host::default();
    let settings = ServerSettings {
        command: root.path().to_string_lossy().into_owned(),
        ..ServerSettings::default()
    };
    let snapshot = snapshot(root.path());
    host.command(
        Command::Open("rust".into(), settings, snapshot.clone()),
        &events,
    );
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::TrustRequired(_)
    ));
    host.command(Command::Trust(snapshot.stamp.id), &events);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Starting)
    ));
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Failed)
    ));
    assert!(matches!(host.session, Session::Closed));
}
