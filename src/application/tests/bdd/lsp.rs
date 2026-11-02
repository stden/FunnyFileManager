//! Workspace trust, cached component errors, explicit retry and warm server reuse.

use super::{lsp_fixture::until, BehaviorWorld};
use crate::models::{lsp::Dialog, ServerState};
use cucumber::{then, when};

#[when(expr = "I edit the Rust document {string}")]
fn edit(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    assert!(case.app.left_panel.select_named(&name));
    case.press("F4");
    case.lsp.pump(&mut case.app);
}

#[when("I trust the fixture language server workspace")]
fn trust(world: &mut BehaviorWorld) {
    let case = world.case();
    until(case, |editor| {
        matches!(editor.lsp.popup, Some(Dialog::Trust { .. }))
    });
    case.press("Down Enter");
    case.lsp.pump(&mut case.app);
}

#[when("the editor reports the missing component")]
fn missing(world: &mut BehaviorWorld) {
    until(world.case(), |editor| {
        editor.lsp.state == ServerState::InstallationFailed
    });
}

#[when("the editor receives fixture diagnostics")]
fn ready(world: &mut BehaviorWorld) {
    until(world.case(), |editor| {
        editor.lsp.state == ServerState::Ready && !editor.lsp.diagnostics.is_empty()
    });
}

#[when("I close the editor before opening another document")]
fn close(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press("Esc");
    assert!(case.app.get_editor().is_none());
    case.lsp.pump(&mut case.app);
}

#[when("I explicitly restart editor language services")]
fn restart(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press("Ctrl+Alt+R");
    case.lsp.pump(&mut case.app);
}

#[when("I remember the language server process")]
fn remember(world: &mut BehaviorWorld) {
    let case = world.case();
    case.lsp_pid = Some(std::fs::read_to_string(case.root.path().join("left/pid")).unwrap());
}

#[then(expr = "the missing-component fixture has launched {int} times")]
fn launches(world: &mut BehaviorWorld, count: usize) {
    let file = world.case().root.path().join("left/spawns");
    let launches = if file.exists() {
        std::fs::read_to_string(file).unwrap().lines().count()
    } else {
        0
    };
    assert_eq!(launches, count);
}

#[then("the same language server process is reused")]
fn reused(world: &mut BehaviorWorld) {
    let case = world.case();
    let pid = std::fs::read_to_string(case.root.path().join("left/pid")).unwrap();
    assert_eq!(Some(pid), case.lsp_pid);
}

#[then(expr = "the editor buffer equals {string}")]
fn buffer(world: &mut BehaviorWorld, text: String) {
    assert_eq!(world.case().app.get_editor().unwrap().content(), text);
}

#[then("the fixture server has shut down cleanly")]
fn shutdown(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.close_overlay();
    case.lsp.pump(&mut case.app);
    drop(std::mem::take(&mut case.lsp));
    let methods = std::fs::read_to_string(case.root.path().join("left/methods")).unwrap();
    assert!(methods.ends_with("exit\n"), "{methods}");
}
