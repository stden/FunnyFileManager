//! Purpose: Verify edits survive installation, failures allow retry, and close cancels installation.

use super::super::*;
use crate::config::{InstallCommand, InstallRecipe};
use std::{
    collections::BTreeMap,
    fs,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn recipe(script: &str) -> ServerSettings {
    ServerSettings {
        command: "missing-ffm-install-test".into(),
        install: Some(InstallRecipe::Command {
            steps: vec![InstallCommand {
                command: "python".into(),
                args: vec!["-c".into(), script.into()],
                env: BTreeMap::new(),
            }],
            executable: "server".into(),
        }),
        ..ServerSettings::default()
    }
}

fn open(
    host: &mut Host,
    settings: ServerSettings,
    snapshot: &Snapshot,
    events: &SyncSender<Envelope>,
) {
    host.command(
        Command::Open("rust".into(), settings, snapshot.clone()),
        events,
    );
    host.command(Command::Trust(snapshot.stamp.id), events);
}

#[test]
fn closing_a_pending_install_discards_edits_and_never_reopens_the_document() {
    let root = TempDir::new().unwrap();
    let mut snapshot = super::super::super::tests::snapshot(root.path());
    let mut host = Host {
        cache: Some(root.path().join("cache")),
        ..Host::default()
    };
    let (events, received) = std::sync::mpsc::sync_channel(32);
    open(
        &mut host,
        recipe("import time; time.sleep(30)"),
        &snapshot,
        &events,
    );
    snapshot.text = "latest edit".into();
    let original = snapshot.path.clone();
    snapshot.path = root.path().join("alias.rs");
    host.command(Command::Update(snapshot.clone()), &events);
    let Session::Preparing(preparing) = &host.session else {
        panic!("installing")
    };
    assert_eq!(preparing.start.snapshot.text, "latest edit");
    assert_eq!(preparing.start.snapshot.path, original);
    let started = Instant::now();
    host.command(Command::Close, &events);
    assert!(started.elapsed() < Duration::from_secs(2));
    host.pump(&events);
    assert!(matches!(host.session, Session::Closed));
    assert!(!received
        .try_iter()
        .any(|event| matches!(event.1, Event::State(ServerState::Ready))));
}

#[test]
fn installer_failure_is_reported_and_leaves_a_closed_session() {
    let root = TempDir::new().unwrap();
    let snapshot = super::super::super::tests::snapshot(root.path());
    let mut host = Host {
        cache: Some(root.path().join("cache")),
        ..Host::default()
    };
    let (events, received) = std::sync::mpsc::sync_channel(32);
    open(&mut host, recipe("raise SystemExit(5)"), &snapshot, &events);
    pump(&mut host, &events, &received, |event| {
        matches!(event, Event::InstallationFailed(_))
    });
    assert!(matches!(host.session, Session::Closed));
}

fn pump(
    host: &mut Host,
    events: &SyncSender<Envelope>,
    received: &Receiver<Envelope>,
    predicate: impl Fn(&Event) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        host.pump(events);
        if received.try_iter().any(|event| predicate(&event.1)) {
            return;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
}

fn cached(root: &Path, settings: &ServerSettings) {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let recipe = serde_json::to_vec(settings.install.as_ref().unwrap()).unwrap();
    let key = Sha256::digest(recipe)
        .iter()
        .fold(String::new(), |mut key, byte| {
            write!(key, "{byte:02x}").unwrap();
            key
        });
    let cache = root.join("cache").join(key);
    fs::create_dir_all(&cache).unwrap();
    let python = std::process::Command::new("python")
        .args(["-c", "import sys; print(sys.executable)"])
        .current_dir(root)
        .output()
        .unwrap();
    let script = root.join("server.py");
    fs::write(&script, include_str!("../../../../tests/server.py")).unwrap();
    let receipt = serde_json::json!({"command": String::from_utf8(python.stdout).unwrap().trim(), "args": [script], "configuration":null});
    fs::write(
        cache.join("ready.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
}

#[test]
fn cached_installation_starts_with_the_latest_document_and_shuts_down_cleanly() {
    let root = TempDir::new().unwrap();
    let settings = recipe("raise SystemExit('cache must bypass installer')");
    cached(root.path(), &settings);
    let mut snapshot = super::super::super::tests::snapshot(root.path());
    let mut host = Host {
        cache: Some(root.path().join("cache")),
        ..Host::default()
    };
    let (events, received) = std::sync::mpsc::sync_channel(32);
    open(&mut host, settings, &snapshot, &events);
    snapshot.text = "latest edit".into();
    host.command(Command::Update(snapshot), &events);
    pump(&mut host, &events, &received, |event| {
        matches!(event, Event::Diagnostics(_, _))
    });
    let Session::Running(connection) = &host.session else {
        panic!("running")
    };
    assert_eq!(
        connection.client.document.as_ref().unwrap().snapshot.text,
        "latest edit"
    );
    host.command(Command::Close, &events);
    assert!(fs::read_to_string(root.path().join("methods"))
        .unwrap()
        .ends_with("shutdown\nexit\n"));
}
