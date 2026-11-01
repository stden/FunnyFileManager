//! Local stdio peers: a missing-component proxy and an existing protocol fixture.

use super::{BehaviorWorld, Case};
use crate::{config::ServerSettings, models::Editor};
use cucumber::given;
use std::{
    fs,
    time::{Duration, Instant},
};

#[given(expr = "a local {word} language server fixture")]
fn configured(world: &mut BehaviorWorld, kind: String) {
    let case = world.case();
    let script = case.root.path().join("server.py");
    let source = match kind.as_str() {
        "missing-component" => {
            include_str!("../../../../tests/behavior_features/missing_component.py")
        }
        "ready" => include_str!("../../../lsp/tests/server.py"),
        _ => panic!("unknown LSP fixture {kind}"),
    };
    fs::write(&script, source).unwrap();
    case.app.config.lsp.0.insert(
        "rust".into(),
        ServerSettings {
            command: "python".into(),
            args: vec![script.to_string_lossy().into_owned()],
            ..ServerSettings::default()
        },
    );
    for name in ["first.rs", "second.rs"] {
        fs::write(case.root.path().join("left").join(name), "fn main() {}").unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
}

pub(super) fn until(case: &mut Case, ready: impl Fn(&Editor) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        case.lsp.pump(&mut case.app);
        if ready(case.app.get_editor().expect("the editor stays open")) {
            case.paint();
            return;
        }
        let editor = case.app.get_editor().unwrap();
        assert!(
            Instant::now() < deadline,
            "fixture LSP did not settle: state={:?}, notice={:?}, launches={:?}",
            editor.lsp.state,
            editor.notice,
            fs::read_to_string(case.root.path().join("left/spawns"))
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
