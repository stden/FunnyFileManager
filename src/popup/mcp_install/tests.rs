//! Unit tests: the Install MCP server dialog's keys, from the menu row to the
//! report, on a fake machine.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::mcp_clients::{ClientId, Scope};
use crate::models::{
    App, Mark, McpInstallDialog, McpInstallField, McpInstallProgress, Overlay, Panel,
};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::status::Status;
use crate::test_support::FakeMachine;

fn open() -> (FakeMachine, App) {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install_clients().unwrap();
    machine.install_server().unwrap();
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    (machine, app)
}

fn dialog(app: &App) -> &McpInstallDialog {
    match &app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the dialog is not open"),
    }
}

fn dialog_mut(app: &mut App) -> &mut McpInstallDialog {
    match &mut app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the dialog is not open"),
    }
}

fn press(app: &mut App, code: KeyCode) {
    assert!(process_popup_input(app, key(code)));
}

#[test]
fn tab_and_the_vertical_arrows_walk_the_controls() {
    let (_machine, mut app) = open();
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        dialog(&app).field,
        McpInstallField::Client(ClientId::ClaudeCode)
    );
    press(&mut app, KeyCode::Down);
    assert_eq!(dialog(&app).field, McpInstallField::Client(ClientId::Codex));
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(dialog(&app).field, McpInstallField::Scope);
}

#[test]
fn left_and_right_switch_scope_only_on_the_radio() {
    let (_machine, mut app) = open();
    press(&mut app, KeyCode::Right);
    assert_eq!(dialog(&app).scope, Scope::CurrentFolder);
    press(&mut app, KeyCode::Left);
    assert_eq!(dialog(&app).scope, Scope::AllProjects);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    assert_eq!(dialog(&app).scope, Scope::AllProjects);
}

#[test]
fn space_toggles_the_focused_client() {
    let (_machine, mut app) = open();
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(dialog(&app).rows[0].mark, Mark::Unchecked);
}

/// Enter only says what is about to run; the event loop draws that, then
/// registers (review finding: `claude` froze the screen without a word).
#[test]
fn enter_announces_the_registration_and_the_loop_runs_it() {
    let (machine, mut app) = open();
    press(&mut app, KeyCode::Enter);
    assert!(crate::ops::mcp_install::is_pending(&app));
    assert_eq!(
        dialog(&app).progress,
        McpInstallProgress::Registering(
            "Registering ffm-mcp: waiting for claude (up to 30 s a call)…".to_owned()
        )
    );
    assert!(machine.claude().calls().is_empty(), "nothing ran yet");
    crate::ops::mcp_install::run_pending(&mut app);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report opens");
    };
    assert_eq!(viewer.title(), "MCP server registration");
    assert!(!crate::ops::mcp_install::is_pending(&app));
}

#[test]
fn enter_on_cancel_cancels() {
    let (machine, mut app) = open();
    dialog_mut(&mut app).field = McpInstallField::Cancel;
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(machine.files().unwrap().is_empty());
}

#[test]
fn enter_with_nothing_checked_stays_open() {
    let (_machine, mut app) = open();
    for row in &mut dialog_mut(&mut app).rows {
        row.mark = Mark::Unchecked;
    }
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::McpInstall(_)));
    assert_eq!(
        app.status,
        Status::Error("Nothing to register: check a client first".to_owned())
    );
}

#[test]
fn enter_without_a_server_closes_the_dialog() {
    let machine = FakeMachine::new().expect("fake machine");
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn esc_and_f10_close_without_changes() {
    for code in [KeyCode::Esc, KeyCode::F(10)] {
        let (machine, mut app) = open();
        press(&mut app, code);
        assert!(matches!(app.overlay, Overlay::None));
        assert!(machine.files().unwrap().is_empty());
    }
}

#[test]
fn another_overlay_is_not_this_dialogs_to_answer() {
    let (_machine, mut app) = open();
    app.close_overlay();
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        KeyModifiers::NONE
    ));
    super::submit(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn other_keys_are_swallowed() {
    let (_machine, mut app) = open();
    let before = dialog(&app).clone();
    assert!(process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
    ));
    assert_eq!(dialog(&app), &before);
}
