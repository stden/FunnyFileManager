//! WSL input queues typed requests; tests never run a system process.

use tempfile::TempDir;

use crate::keymap::{tables, ActionName, Explicit};
use crate::models::{DistributionName, WslDialog, WslOperation, WslRequestId};
use crate::test_support::chord;

use super::*;

fn app() -> App {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(vec![
        DistributionName::new("Ubuntu").expect("distribution"),
        DistributionName::new("Debian").expect("distribution"),
        DistributionName::new("Alpine").expect("distribution"),
    ]))));
    app
}

fn dialog(app: &App) -> &WslDialog {
    let Overlay::Wsl(dialog) = &app.overlay else {
        panic!("WSL dialog must remain open");
    };
    dialog
}

fn key(app: &mut App, code: KeyCode) {
    assert!(process(app, code, KeyModifiers::NONE));
}

/// Presses the key written `text`, as a terminal sends it.
fn press(app: &mut App, text: &str) {
    let event = chord(text).to_event();
    assert!(process(app, event.code, event.modifiers), "{text}");
}

#[test]
fn navigation_selects_distributions_without_queuing_work() {
    let mut app = app();
    key(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).selected(), Some(1));
    key(&mut app, KeyCode::End);
    assert_eq!(dialog(&app).selected(), Some(2));
    key(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).selected(), Some(2));
    key(&mut app, KeyCode::Up);
    assert_eq!(dialog(&app).selected(), Some(1));
    key(&mut app, KeyCode::Home);
    assert_eq!(dialog(&app).selected(), Some(0));
    assert_eq!(app.wsl_requests.pending().len(), 0);
}

#[test]
fn operation_keys_queue_distinct_platform_and_distribution_requests() {
    let name = DistributionName::new("Debian").expect("distribution");
    for (code, operation) in [
        (KeyCode::Enter, WslOperation::Open(name.clone())),
        (KeyCode::F(2), WslOperation::Refresh),
        (KeyCode::F(5), WslOperation::Update),
        (KeyCode::F(6), WslOperation::UpdatePackages(name)),
    ] {
        let mut app = app();
        key(&mut app, KeyCode::Down);
        key(&mut app, code);
        assert_eq!(app.wsl_requests.pending().len(), 1);
        assert_eq!(app.wsl_requests.pending()[0].operation, operation);
        assert!(matches!(dialog(&app).activity, WslActivity::Busy { .. }));
    }
}

#[test]
fn busy_dialog_consumes_new_operations_but_allows_navigation_and_close() {
    let mut app = app();
    let Overlay::Wsl(state) = &mut app.overlay else {
        panic!("the WSL dialog is open");
    };
    state.activity = WslActivity::Busy {
        id: WslRequestId::new(7),
        operation: WslOperation::Update,
    };
    for code in [KeyCode::Enter, KeyCode::F(2), KeyCode::F(5), KeyCode::F(6)] {
        key(&mut app, code);
    }
    assert_eq!(app.wsl_requests.pending().len(), 0);
    key(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).selected(), Some(1));
    assert!(matches!(
        dialog(&app).activity,
        WslActivity::Busy {
            operation: WslOperation::Update,
            ..
        }
    ));
    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn unsupported_keys_never_reach_the_files_behind_the_dialog() {
    let temp = TempDir::new().expect("temporary directory");
    let file = temp.path().join("keep.txt");
    std::fs::write(&file, "kept").expect("file");
    let mut app = app();
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.left_panel.select_named("keep.txt");
    key(&mut app, KeyCode::F(8));
    assert_eq!(std::fs::read_to_string(file).expect("file remains"), "kept");
    assert_eq!(dialog(&app).selected(), Some(0));
    assert_eq!(app.wsl_requests.pending().len(), 0);
    app.set_overlay(Overlay::Help);
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.overlay, Overlay::Help));
}

/// Exact matching: a modifier makes another key, which the dialog consumes
/// and answers with nothing.
#[test]
fn a_key_held_with_a_modifier_stays_in_the_dialog_and_does_nothing() {
    let mut app = app();
    for text in ["Ctrl+Down", "Shift+End", "Shift+Enter", "Alt+F5", "Alt+Esc"] {
        press(&mut app, text);
        assert_eq!(dialog(&app).selected(), Some(0), "{text}");
        assert_eq!(app.wsl_requests.pending().len(), 0, "{text}");
    }
}

#[test]
fn the_dialog_reads_the_keys_the_app_holds() {
    let file = Explicit::from([
        (ActionName::new("cursor_down"), vec![chord("j")]),
        (ActionName::new("refresh"), vec![chord("Ctrl+R")]),
    ]);
    let mut app = app();
    app.keymap.wsl_manager = tables::WSL_MANAGER.layered(&file).bindings;
    press(&mut app, "Down");
    press(&mut app, "F2");
    assert_eq!(dialog(&app).selected(), Some(0));
    assert_eq!(app.wsl_requests.pending().len(), 0);
    press(&mut app, "j");
    press(&mut app, "Ctrl+R");
    assert_eq!(dialog(&app).selected(), Some(1));
    assert_eq!(
        app.wsl_requests.pending()[0].operation,
        WslOperation::Refresh
    );
}
