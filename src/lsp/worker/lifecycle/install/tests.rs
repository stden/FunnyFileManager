//! Purpose: Verify installation cannot run before trust and reports missing storage gracefully.

use super::*;
use crate::lsp::manager::Command;
use std::path::Path;
use tempfile::TempDir;

mod transitions;

fn absent_python_settings(root: &Path) -> ServerSettings {
    let mut settings = crate::config::LspSettings::default()
        .0
        .remove("python")
        .unwrap();
    settings.command = root.join("absent").to_string_lossy().into_owned();
    settings
}

#[allow(
    clippy::too_many_lines,
    reason = "integration test exercises a complete trust-install-cache workflow that cannot be split without duplicating setup"
)]
#[test]
fn trusted_missing_servers_report_and_cache_missing_runtime_until_restart() {
    let root = TempDir::new().unwrap();
    let mut snapshot = super::super::tests::snapshot(root.path());
    snapshot.path = root.path().join("source.py");
    let settings = absent_python_settings(root.path());
    let (events, received) = std::sync::mpsc::sync_channel(8);
    let mut host = Host::default();
    host.command(
        Command::Open("python".into(), settings.clone(), snapshot.clone()),
        &events,
    );
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::TrustRequired(_)
    ));
    assert!(!host.is_active());
    host.command(Command::Trust(snapshot.stamp.id), &events);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Starting)
    ));
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::InstallationFailed(_)
    ));
    assert_ne!(host.missing.len(), 0);
    host.command(Command::Open("python".into(), settings, snapshot), &events);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::InstallationFailed(_)
    ));
    host.command(Command::Restart, &events);
    assert_eq!(host.missing.len(), 0);
}

fn rustup_shim_settings(marker: &Path) -> ServerSettings {
    let script = format!(
        "import pathlib, sys; p = pathlib.Path(r'{}'); p.write_text((p.read_text() if p.exists() else '') + '1'); sys.stderr.write(\"error: 'rust-analyzer.exe' is not installed for the toolchain '1.99.1'\\n\"); raise SystemExit(1)",
        marker.display()
    );
    ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), script],
        ..ServerSettings::default()
    }
}

#[test]
fn rustup_missing_component_reports_install_hint_and_caches_until_restart() {
    let root = TempDir::new().unwrap();
    let snapshot = super::super::tests::snapshot(root.path());
    let marker = root.path().join("spawns");
    let settings = rustup_shim_settings(&marker);
    let (events, received) = std::sync::mpsc::sync_channel(16);
    let mut host = Host::default();
    host.command(
        Command::Open("rust".into(), settings.clone(), snapshot.clone()),
        &events,
    );
    host.command(Command::Trust(snapshot.stamp.id), &events);
    assert!(host.is_active());
    transitions::pump(&mut host, &events, &received, |event| {
        matches!(
            event,
            Event::InstallationFailed(msg) if msg.contains("rustup component add rust-analyzer")
        )
    });
    assert!(!host.is_active());
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "1");
    host.command(Command::Open("rust".into(), settings, snapshot), &events);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::InstallationFailed(msg) if msg.contains("rustup component add rust-analyzer")
    ));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "1");
}

fn idle_python_settings() -> ServerSettings {
    ServerSettings {
        command: "python".into(),
        args: vec!["-c".into(), "import time; time.sleep(0.05)".into()],
        ..ServerSettings::default()
    }
}

#[test]
fn park_evicts_oldest_idle_server_restart_prunes_matches_and_reopen_reports_initializing() {
    let root = TempDir::new().unwrap();
    let snap = super::super::tests::snapshot(root.path());
    let (events, received) = std::sync::mpsc::sync_channel(32);
    let cfg = idle_python_settings();
    let mut host = Host::default();
    for idx in 0..5 {
        let mut conn =
            Connection::start(format!("lang{idx}"), &cfg, snap.clone(), root.path()).unwrap();
        conn.reopen(snap.clone(), &events);
        host.session = Session::Running(Box::new(conn));
        host.park();
    }
    assert_eq!(host.idle.len(), 4);
    assert!(matches!(
        received.try_recv().unwrap().1,
        Event::State(ServerState::Initializing)
    ));

    let active = Connection::start("lang1".into(), &cfg, snap, root.path()).unwrap();
    host.session = Session::Running(Box::new(active));
    host.restart();
    assert_eq!(host.idle.len(), 3);
}

#[test]
fn parking_a_fifth_server_evicts_the_oldest_idle_connection() {
    let root = TempDir::new().unwrap();
    let snapshot = super::super::tests::snapshot(root.path());
    let settings = idle_python_settings();
    let mut host = Host::default();
    for index in 0..5 {
        let language = format!("lang{index}");
        let connection =
            Connection::start(language, &settings, snapshot.clone(), root.path()).unwrap();
        host.session = Session::Running(Box::new(connection));
        host.park();
    }
    let parked: Vec<_> = host.idle.iter().map(|item| &item.language).collect();
    assert_eq!(parked, ["lang1", "lang2", "lang3", "lang4"]);
    assert!(matches!(host.session, Session::Closed));
}
