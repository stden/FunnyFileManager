//! Unit tests for [`super`]: opening the dialog on the active panel's folder,
//! switching scope, and registering the checked clients through an `App` on
//! a fake machine.

use std::fs;

use crate::mcp_clients::{ConfigState, ServerState};
use crate::models::{McpInstallField, Panel};
use crate::test_support::{ClaudeBehaviour, FakeMachine};

use super::*;

/// A machine with every client installed and the server beside `ffm`, and
/// an `App` whose panels show its project folder.
fn machine_app() -> (FakeMachine, App) {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install_clients().unwrap();
    machine.install_server().unwrap();
    let app = app_on(&machine);
    (machine, app)
}

fn app_on(machine: &FakeMachine) -> App {
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    app
}

fn dialog(app: &App) -> &McpInstallDialog {
    match &app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the Install MCP server dialog is not open"),
    }
}

fn dialog_mut(app: &mut App) -> &mut McpInstallDialog {
    match &mut app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the Install MCP server dialog is not open"),
    }
}

#[test]
fn a_new_app_looks_at_no_machine_at_all() {
    let app = App::with_panels(
        Panel::new(std::env::temp_dir()).unwrap(),
        Panel::new(std::env::temp_dir()).unwrap(),
    );
    assert_eq!(app.live.mcp_host.home, None);
    assert!(app.live.mcp_host.search_path.is_empty());
    assert_eq!(app.live.mcp_host.exe_dir, None);
    assert!(app.live.mcp_host.env.is_empty());
}

#[test]
fn open_surveys_all_projects_on_the_active_panels_folder() {
    let (machine, mut app) = machine_app();
    open(&mut app);
    let dialog = dialog(&app);
    assert_eq!(dialog.scope, Scope::AllProjects);
    assert_eq!(
        dialog
            .folder
            .as_ref()
            .map(|folder| folder.as_path().to_path_buf()),
        Some(machine.folder())
    );
    assert!(matches!(dialog.server, ServerState::Found(_)));
    assert_eq!(dialog.rows.len(), 4);
    assert_eq!(dialog.field, McpInstallField::Scope);
}

/// A `NetBox` panel's path still names the local folder it hides: the
/// dialog opens with no folder, so Current folder is not offered for it.
#[test]
fn open_over_a_netbox_panel_has_no_folder() {
    let (_machine, mut app) = machine_app();
    let side = app.active_panel;
    crate::test_support::netbox::open_empty_sessions(&mut app, side);
    assert_eq!(active_folder(&app), None);
    open(&mut app);
    assert_eq!(dialog(&app).folder, None);
    switch_scope(&mut app, Direction::Forward);
    assert_eq!(dialog(&app).scope, Scope::AllProjects);
}

#[test]
fn switching_scope_surveys_the_folder_and_back() {
    let (machine, mut app) = machine_app();
    open(&mut app);
    switch_scope(&mut app, Direction::Forward);
    assert_eq!(dialog(&app).scope, Scope::CurrentFolder);
    let codex = &dialog(&app).rows[1];
    assert_eq!(
        codex.status.config,
        Some(machine.folder().join(".codex").join("config.toml"))
    );
    switch_scope(&mut app, Direction::Backward);
    assert_eq!(dialog(&app).scope, Scope::AllProjects);
}

#[test]
fn switching_resets_the_marks_to_the_new_scopes_defaults() {
    let (_machine, mut app) = machine_app();
    open(&mut app);
    dialog_mut(&mut app).rows[2].mark = Mark::Unchecked;
    switch_scope(&mut app, Direction::Forward);
    assert!(dialog(&app)
        .rows
        .iter()
        .all(|row| row.mark == Mark::Checked));
}

#[test]
fn switching_does_nothing_without_a_folder_or_a_dialog() {
    let (_machine, mut app) = machine_app();
    switch_scope(&mut app, Direction::Forward);
    assert!(matches!(app.overlay, Overlay::None));
    open(&mut app);
    dialog_mut(&mut app).folder = None;
    switch_scope(&mut app, Direction::Forward);
    assert_eq!(dialog(&app).scope, Scope::AllProjects);
}

#[test]
fn register_writes_every_checked_client_and_shows_the_report() {
    let (machine, mut app) = machine_app();
    open(&mut app);
    register(&mut app);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report opens");
    };
    assert_eq!(viewer.title(), "MCP server registration");
    assert_eq!(
        app.status,
        Status::CommandOutput("ffm-mcp: 4 added".to_owned())
    );
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        assert!(
            machine.config(client, Scope::AllProjects).is_file(),
            "{client:?}"
        );
    }
    assert_eq!(machine.claude().calls().len(), 1);
}

#[test]
fn register_in_the_current_folder_roots_every_entry_there() {
    let (machine, mut app) = machine_app();
    open(&mut app);
    switch_scope(&mut app, Direction::Forward);
    register(&mut app);
    let folder = machine.folder().display().to_string();
    for client in [
        ClientId::ClaudeCode,
        ClientId::Codex,
        ClientId::Junie,
        ClientId::Kiro,
    ] {
        let text = fs::read_to_string(machine.config(client, Scope::CurrentFolder)).unwrap();
        assert!(text.contains("FFM_MCP_ROOT"), "{client:?}: {text}");
        assert!(
            text.contains(&folder.replace('\\', "\\\\")) || text.contains(&folder),
            "{text}"
        );
    }
    assert!(machine.claude().calls().is_empty());
}

#[test]
fn one_client_failing_still_registers_the_rest() {
    let (machine, mut app) = machine_app();
    machine.claude().behave(ClaudeBehaviour::TimeOut);
    open(&mut app);
    register(&mut app);
    assert_eq!(
        app.status,
        Status::Error("ffm-mcp: Claude Code failed; see the report".to_owned())
    );
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        assert!(
            machine.config(client, Scope::AllProjects).is_file(),
            "{client:?}"
        );
    }
}

#[test]
fn nothing_checked_keeps_the_dialog_open_and_says_so() {
    let (_machine, mut app) = machine_app();
    open(&mut app);
    for row in &mut dialog_mut(&mut app).rows {
        row.mark = Mark::Unchecked;
    }
    register(&mut app);
    assert!(matches!(app.overlay, Overlay::McpInstall(_)));
    assert_eq!(app.status, Status::Error(NOTHING_CHECKED.to_owned()));
}

#[test]
fn register_without_a_server_only_closes_the_dialog() {
    let (machine, mut app) = machine_app();
    machine.remove_server().unwrap();
    let before = machine.files().unwrap();
    open(&mut app);
    assert_eq!(dialog(&app).server, ServerState::Missing);
    register(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(machine.files().unwrap(), before);
}

#[test]
fn register_without_the_dialog_does_nothing() {
    let (machine, mut app) = machine_app();
    register(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(machine.files().unwrap().is_empty());
}

#[test]
fn a_second_registration_changes_nothing() {
    let (machine, mut app) = machine_app();
    open(&mut app);
    register(&mut app);
    let before = machine.files().unwrap();
    open(&mut app);
    assert!(dialog(&app)
        .rows
        .iter()
        .all(|row| row.status.state == ConfigState::Registered));
    register(&mut app);
    assert_eq!(
        app.status,
        Status::CommandOutput("ffm-mcp: 4 unchanged".to_owned())
    );
    assert_eq!(machine.files().unwrap(), before);
}

#[test]
fn the_menu_row_opens_the_dialog() {
    let (_machine, mut app) = machine_app();
    crate::ops::menu::execute(&mut app, crate::menu::MenuCommand::McpInstall);
    assert!(matches!(app.overlay, Overlay::McpInstall(_)));
}

/// The dialog at Current folder with the panels on the fake home folder.
fn current_folder_at_home() -> (FakeMachine, App) {
    let (machine, mut app) = machine_app();
    for panel in [&mut app.left_panel, &mut app.right_panel] {
        *panel = Panel::new(machine.home()).unwrap();
    }
    open(&mut app);
    switch_scope(&mut app, Direction::Forward);
    (machine, app)
}

/// Review finding: with the panel at the home folder, Current folder named
/// Codex's, Junie's and Kiro's user-level configs and wrote them with
/// `FFM_MCP_ROOT` set to the home. Those rows are now dimmed.
#[test]
fn current_folder_at_the_home_folder_dims_the_shared_configs() {
    let (_machine, app) = current_folder_at_home();
    let dialog = dialog(&app);
    let dimmed: Vec<ClientId> = dialog
        .rows
        .iter()
        .filter(|row| !dialog.is_enabled(row))
        .map(|row| row.status.client)
        .collect();
    assert_eq!(dimmed, [ClientId::Codex, ClientId::Junie, ClientId::Kiro]);
}

/// ... and a registration forced through them writes nothing there.
#[test]
fn current_folder_at_the_home_folder_leaves_the_user_configs_alone() {
    let (machine, mut app) = current_folder_at_home();
    for row in &mut dialog_mut(&mut app).rows {
        row.mark = Mark::Checked;
    }
    register(&mut app);
    let Overlay::Viewer(viewer) = &app.overlay else {
        panic!("the report opens");
    };
    let report = viewer.lines.join("\n");
    assert!(
        report.contains("is also the All projects config"),
        "{report}"
    );
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        let config = machine.config(client, Scope::AllProjects);
        assert!(!config.exists(), "{client:?}: {report}");
    }
    assert!(machine.home().join(".mcp.json").is_file(), "Claude's own");
}

#[test]
fn a_registration_without_claude_is_announced_plainly() {
    let (machine, mut app) = machine_app();
    run_pending(&mut app);
    assert!(matches!(app.overlay, Overlay::None), "no dialog, no run");
    open(&mut app);
    switch_scope(&mut app, Direction::Forward);
    run_pending(&mut app);
    assert!(
        matches!(app.overlay, Overlay::McpInstall(_)),
        "nothing waits"
    );
    submit(&mut app);
    assert_eq!(
        dialog(&app).progress,
        McpInstallProgress::Registering("Registering ffm-mcp…".to_owned())
    );
    run_pending(&mut app);
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    assert!(machine.claude().calls().is_empty());
}

/// A registration that stops early leaves the dialog open, and must not run
/// again on every event after.
#[test]
fn a_pending_registration_runs_once_even_when_it_stops_early() {
    let (_machine, mut app) = machine_app();
    open(&mut app);
    submit(&mut app);
    for row in &mut dialog_mut(&mut app).rows {
        row.mark = Mark::Unchecked;
    }
    run_pending(&mut app);
    assert_eq!(app.status, Status::Error(NOTHING_CHECKED.to_owned()));
    assert_eq!(dialog(&app).progress, McpInstallProgress::Choosing);
    assert!(!is_pending(&app));
}
