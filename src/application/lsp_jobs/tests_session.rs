//! Purpose: Exercise the editor controller, popup keys, save, restart, and close against a fake process.

use super::*;
use crate::{
    config::ServerSettings,
    models::{
        lsp::{Dialog, Intent},
        Editor, LspFeature, Overlay, Panel,
    },
};
use std::{fs, thread};
use tempfile::TempDir;

fn fixture(root: &TempDir) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    let script = root.path().join("server.py");
    fs::write(&script, include_str!("../../lsp/tests/server.py")).unwrap();
    app.config.lsp.0.insert(
        "rust".into(),
        ServerSettings {
            command: "python".into(),
            args: vec![script.to_string_lossy().into_owned()],
            ..ServerSettings::default()
        },
    );
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "main.rs".into(),
        root.path().join("main.rs"),
        "fn main() {}",
    )));
    app
}

fn pump_until(
    controller: &mut EditorLspController,
    app: &mut App,
    ready: impl Fn(&Editor) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        controller.pump(app);
        if ready(app.get_editor().unwrap()) {
            return;
        }
        assert!(Instant::now() < deadline, "Editor LSP timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn editor_debounces_changes_saves_and_restarts_without_losing_text() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let mut controller = EditorLspController::default();
    pump_until(&mut controller, &mut app, |editor| {
        !editor.lsp.diagnostics.is_empty()
    });
    app.get_editor_mut().unwrap().insert_char('x');
    controller.pump(&mut app);
    assert_ne!(
        controller.sent.unwrap().0,
        app.get_editor().unwrap().lsp_stamp()
    );
    pump_until(&mut controller, &mut app, |editor| {
        !editor.lsp.diagnostics.is_empty()
    });
    app.get_editor_mut().unwrap().save().unwrap();
    controller.pump(&mut app);
    assert_eq!(controller.sent.unwrap().1, 1);
    let text = app.get_editor().unwrap().content();
    app.get_editor_mut().unwrap().lsp.intent = Intent::Restart;
    controller.pump(&mut app);
    pump_until(&mut controller, &mut app, |editor| {
        editor.lsp.state == ServerState::Ready
    });
    assert_eq!(app.get_editor().unwrap().content(), text);
    close(&mut controller, &mut app);
    drop(controller);
    assert!(fs::read_to_string(root.path().join("methods"))
        .unwrap()
        .ends_with("exit\n"));
}

fn close(controller: &mut EditorLspController, app: &mut App) {
    app.close_overlay();
    controller.pump(app);
    assert!(controller.observed.is_none());
}

#[test]
fn editor_receives_completion_hover_and_definition_from_the_background_worker() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    let mut controller = EditorLspController::default();
    pump_until(&mut controller, &mut app, |editor| {
        editor.lsp.state == ServerState::Ready
    });
    for feature in [LspFeature::Completion, LspFeature::Hover] {
        app.get_editor_mut().unwrap().lsp.intent = Intent::Query(feature);
        pump_until(&mut controller, &mut app, |editor| {
            editor.lsp.popup.is_some()
        });
        let dialog = app.get_editor_mut().unwrap().lsp.popup.take().unwrap();
        assert!(matches!(
            dialog,
            Dialog::Completion { .. } | Dialog::Hover { .. }
        ));
    }
    let editor = app.get_editor_mut().unwrap();
    editor.cursor_col = 5;
    editor.lsp.intent = Intent::Query(LspFeature::Definition);
    pump_until(&mut controller, &mut app, |editor| editor.cursor_col == 0);
}
#[test]
#[ignore = "explicit performance probe; timings are machine dependent"]
fn measure_editor_startup_and_typing_while_the_server_is_busy() {
    let root = TempDir::new().unwrap();
    let mut app = fixture(&root);
    app.config.lsp.0.get_mut("rust").unwrap().enabled = false;
    let start = Instant::now();
    let mut controller = EditorLspController::default();
    controller.pump(&mut app);
    eprintln!("editor startup without LSP: {:?}", start.elapsed());
    let settings = app.config.lsp.0.get_mut("rust").unwrap();
    settings.enabled = true;
    settings.args.push("busy".into());
    controller.pump(&mut app);
    let typing = Instant::now();
    for _ in 0..1000 {
        app.get_editor_mut().unwrap().insert_char('x');
        controller.pump(&mut app);
    }
    eprintln!(
        "1000 edits and event-loop pumps during initialization: {:?}",
        typing.elapsed()
    );
    assert_eq!(app.get_editor().unwrap().lines[0].chars().count(), 1012);
    assert!(typing.elapsed() < Duration::from_secs(2));
}
