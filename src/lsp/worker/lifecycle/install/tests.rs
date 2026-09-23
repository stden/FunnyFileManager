//! Purpose: Verify installation cannot run before trust and reports missing storage gracefully.

use super::*;
use crate::lsp::manager::Command;
use tempfile::TempDir;

mod transitions;

#[test]
fn trusted_missing_servers_report_installation_failure_when_storage_is_unavailable() {
    let root = TempDir::new().unwrap();
    let mut snapshot = super::super::tests::snapshot(root.path());
    snapshot.path = root.path().join("source.py");
    let mut settings = crate::config::LspSettings::default()
        .0
        .remove("python")
        .unwrap();
    settings.command = root.path().join("absent").to_string_lossy().into_owned();
    let (events, received) = std::sync::mpsc::sync_channel(8);
    let mut host = Host::default();
    host.command(
        Command::Open("python".into(), settings, snapshot.clone()),
        &events,
    );
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::TrustRequired(_)
    ));
    assert!(matches!(host.session, Session::Untrusted(_)));
    host.command(Command::Trust(snapshot.stamp.id), &events);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Starting)
    ));
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::InstallationFailed(_)
    ));
}
