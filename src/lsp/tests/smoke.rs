//! Purpose: Optional real rust-analyzer acceptance and latency test in an isolated Rust workspace.

use super::super::{
    client::Event,
    features::Reply,
    manager::{Command, Manager},
    positions::Position,
    types::{CompletionTrigger, DocumentId, Feature, Query, Revision, Snapshot, Stamp},
};
use crate::config::ServerSettings;
use std::{
    fs, thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const SOURCE: &str = "fn local_value() -> u32 { 42 }\nfn main() {\n    let broken = ;\n    let result = local_value();\n    let _ = result;\n}\n";

fn fixture() -> (TempDir, Snapshot) {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"lsp_smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    let path = root.path().join("src/main.rs");
    fs::write(&path, SOURCE).unwrap();
    let snapshot = Snapshot {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision(0),
        },
        path,
        text: SOURCE.into(),
        saved: 0,
    };
    (root, snapshot)
}

fn wait(manager: &Manager, predicate: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(120);
    wait_until(manager, predicate, deadline)
}

fn wait_until(manager: &Manager, predicate: impl Fn(&Event) -> bool, deadline: Instant) -> Event {
    loop {
        if let Some(event) = manager.poll() {
            if matches!(event.1, Event::TrustRequired(_)) {
                assert!(manager.send(Command::Trust(event.0)).is_ok());
            }
            assert!(
                !matches!(
                    event.1,
                    Event::State(
                        super::super::types::ServerState::Failed
                            | super::super::types::ServerState::Unavailable
                    )
                ),
                "rust-analyzer failed"
            );
            if predicate(&event.1) {
                return event.1;
            }
        }
        assert!(
            Instant::now() < deadline,
            "rust-analyzer acceptance timed out"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn request(manager: &Manager, snapshot: &Snapshot, feature: Feature, character: u32) -> Reply {
    let deadline = Instant::now() + Duration::from_secs(120);
    let query = Query {
        stamp: snapshot.stamp,
        position: Position { line: 3, character },
        feature,
    };
    for _ in 0..30 {
        assert!(manager.send(Command::Query(query)).is_ok());
        match wait_until(
            manager,
            |event| matches!(event, Event::Reply(_, _)),
            deadline,
        ) {
            Event::Reply(_, Ok(reply)) => return reply,
            Event::Reply(_, Err(error)) if error.code == -32801 => {
                thread::sleep(Duration::from_millis(200));
            }
            event => panic!("Request failed: {event:?}"),
        }
    }
    panic!("rust-analyzer remained busy after bounded retries")
}

#[test]
#[ignore = "requires an installed rust-analyzer; run explicitly for acceptance"]
fn real_rust_analyzer_supports_the_complete_editor_protocol() {
    let (_root, mut snapshot) = fixture();
    let start = Instant::now();
    let manager = Manager::start().unwrap();
    assert!(manager
        .send(Command::Open(
            "rust".into(),
            ServerSettings::default(),
            snapshot.clone()
        ))
        .is_ok());
    eprintln!("background startup handoff: {:?}", start.elapsed());
    wait(
        &manager,
        |event| matches!(event, Event::Diagnostics(_, diagnostics) if !diagnostics.is_empty()),
    );
    eprintln!("startup to diagnostics: {:?}", start.elapsed());
    verify_features(&manager, &snapshot);
    verify_editing(&manager, &mut snapshot);
    let shutdown = Instant::now();
    drop(manager);
    eprintln!("shutdown: {:?}", shutdown.elapsed());
    assert!(shutdown.elapsed() < Duration::from_secs(4));
}

fn verify_editing(manager: &Manager, snapshot: &mut Snapshot) {
    snapshot.stamp.revision = Revision(1);
    snapshot.text = SOURCE.replace("let broken = ;", "let _broken = 1;");
    // Keep disk and buffer aligned; this assertion checks diagnostic refresh.
    // Deterministic client tests separately verify the didSave notification itself.
    fs::write(&snapshot.path, &snapshot.text).unwrap();
    snapshot.saved += 1;
    let typing = Instant::now();
    assert!(manager.send(Command::Update(snapshot.clone())).is_ok());
    eprintln!("typing handoff: {:?}", typing.elapsed());
    wait(
        manager,
        |event| matches!(event, Event::Diagnostics(stamp, diagnostics) if *stamp == snapshot.stamp && diagnostics.iter().all(|diagnostic| diagnostic.severity != Some(1))),
    );
}

fn verify_features(manager: &Manager, snapshot: &Snapshot) {
    let start = Instant::now();
    let Reply::Completions(items) = request(
        manager,
        snapshot,
        Feature::Completion(CompletionTrigger::Manual),
        24,
    ) else {
        panic!("completion shape")
    };
    assert!(
        items.iter().any(|item| item.label.contains("local_value")),
        "no useful completion: {items:?}"
    );
    eprintln!("completion latency: {:?}", start.elapsed());
    let Reply::Hover(text) = request(manager, snapshot, Feature::Hover, 22) else {
        panic!("hover shape")
    };
    assert!(text.contains("local_value"), "hover: {text}");
    let Reply::Definitions(items) = request(manager, snapshot, Feature::Definition, 22) else {
        panic!("definition shape")
    };
    assert!(
        items.iter().any(
            |item| super::super::uri::same_path(&item.path, &snapshot.path)
                && item.range.start.line == 0
        ),
        "definition results: {items:?}"
    );
}
