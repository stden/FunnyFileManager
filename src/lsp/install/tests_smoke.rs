//! Purpose: Optional real package installation and stdio acceptance in disposable workspaces.

use super::*;
use crate::{
    config::LspSettings,
    lsp::{
        client::Event,
        features::Reply,
        manager::{Command, Manager},
        positions::Position,
        types::{
            CompletionTrigger, DocumentId, Feature, Query, Revision, ServerState, Snapshot, Stamp,
        },
    },
};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const PYTHON: &str = "def local_value():\n    \"\"\"Return a value.\"\"\"\n    return 42\nresult = local_value()\nmissing_name\n";
const JAVA: &str = "class Main {\n    static int localValue() { return 42; }\n    public static void main(String[] args) {\n        int result = localValue();\n        int broken = ;\n    }\n}\n";

#[test]
#[ignore = "downloads Python LSP; requires Python with venv/pip and network"]
fn real_python_package_supports_editor_features() {
    acceptance("python", "main.py", PYTHON, 14, "missing_name", "result");
}

#[test]
#[ignore = "downloads JDT LS; requires Java 21+, curl and network"]
fn real_java_package_supports_editor_features() {
    acceptance(
        "java",
        "Main.java",
        JAVA,
        24,
        "int broken = ;",
        "int broken = 1;",
    );
}

fn snapshot(root: &Path, file: &str, text: &str) -> Snapshot {
    let path = root.join(file);
    fs::write(&path, text).unwrap();
    Snapshot {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision(0),
        },
        path,
        text: text.into(),
        saved: 0,
    }
}

fn acceptance(language: &str, file: &str, text: &str, column: u32, broken: &str, fixed: &str) {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let settings = LspSettings::default().0.remove(language).unwrap();
    let started = Instant::now();
    let installed = storage::prepare(
        &settings,
        cache.path(),
        root.path(),
        &AtomicBool::new(false),
    )
    .unwrap();
    eprintln!("{language} installation: {:?}", started.elapsed());
    let mut snapshot = snapshot(root.path(), file, text);
    let manager = Manager::start().unwrap();
    assert!(manager
        .send(Command::Open(language.into(), installed, snapshot.clone()))
        .is_ok());
    wait(&manager, |event| {
        matches!(event, Event::State(ServerState::Ready))
    });
    eprintln!("{language} initialized: {:?}", started.elapsed());
    wait(&manager, has_errors);
    eprintln!("{language} ready + diagnostics: {:?}", started.elapsed());
    verify_features(&manager, &snapshot, column);
    verify_edit(&manager, &mut snapshot, &text.replace(broken, fixed));
    let shutdown = Instant::now();
    drop(manager);
    eprintln!("{language} shutdown: {:?}", shutdown.elapsed());
    assert!(shutdown.elapsed() < Duration::from_secs(4));
}

fn verify_edit(manager: &Manager, snapshot: &mut Snapshot, text: &str) {
    snapshot.stamp.revision = Revision(1);
    snapshot.text = text.into();
    snapshot.saved = 1;
    fs::write(&snapshot.path, &snapshot.text).unwrap();
    assert!(manager.send(Command::Update(snapshot.clone())).is_ok());
    wait(manager, |event| {
        matches!(event, Event::Diagnostics(_, _)) && !has_errors(event)
    });
}

fn has_errors(event: &Event) -> bool {
    matches!(event, Event::Diagnostics(_, items) if items.iter().any(|item| item.severity == Some(1)))
}

fn wait(manager: &Manager, predicate: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(event) = manager.poll() {
            if matches!(event.1, Event::TrustRequired(_)) {
                assert!(manager.send(Command::Trust(event.0)).is_ok());
            }
            assert!(
                !matches!(
                    event.1,
                    Event::State(
                        ServerState::Failed
                            | ServerState::Unavailable
                            | ServerState::InstallationFailed
                    ) | Event::InstallationFailed(_)
                ),
                "server failed: {:?}",
                event.1
            );
            if predicate(&event.1) {
                return event.1;
            }
        }
        assert!(Instant::now() < deadline, "acceptance timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

fn verify_features(manager: &Manager, snapshot: &Snapshot, column: u32) {
    for feature in [
        Feature::Hover,
        Feature::Definition,
        Feature::Completion(CompletionTrigger::Manual),
    ] {
        let query = Query {
            stamp: snapshot.stamp,
            position: Position {
                line: 3,
                character: column,
            },
            feature,
        };
        let started = Instant::now();
        assert!(manager.send(Command::Query(query)).is_ok());
        let reply = wait(manager, |event| matches!(event, Event::Reply(_, _)));
        match reply {
            Event::Reply(_, Ok(Reply::Hover(text))) => assert!(!text.is_empty()),
            Event::Reply(_, Ok(Reply::Definitions(items))) => assert!(!items.is_empty()),
            Event::Reply(_, Ok(Reply::Completions(items))) => assert!(!items.is_empty()),
            other => panic!("unexpected response: {other:?}"),
        }
        eprintln!("{feature:?}: {:?}", started.elapsed());
    }
}
