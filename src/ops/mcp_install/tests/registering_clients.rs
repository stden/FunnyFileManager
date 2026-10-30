//! Purpose: Register: every checked client, failures, nothing checked, and a second run.

use super::*;

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
    assert_eq!(machine.claude().calls().len(), 0);
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
    assert_eq!(machine.files().unwrap().len(), 0);
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

/// Current folder writes into the folder the panels show: the new
/// `.mcp.json`, `.codex`, `.junie` and `.kiro` are listed at once, not only
/// after Ctrl+R.
#[test]
fn register_in_the_current_folder_refreshes_the_panels() {
    // Another test forces refresh_panels to fail while it holds this lock.
    let _refresh = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_machine, mut app) = machine_app();
    open(&mut app);
    switch_scope(&mut app, Direction::Forward);
    register(&mut app);
    for panel in [&app.left_panel, &app.right_panel] {
        let names: Vec<&str> = panel
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        for name in [".mcp.json", ".codex", ".junie", ".kiro"] {
            assert!(names.contains(&name), "{name} in {names:?}");
        }
    }
}

/// Panels that cannot be read again do not change what registering did:
/// the report opens and the status is still the registration's.
#[test]
fn a_panel_refresh_that_fails_keeps_the_registrations_status() {
    let _refresh = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_machine, mut app) = machine_app();
    open(&mut app);
    crate::utils::set_refresh_panels_error(true);
    register(&mut app);
    crate::utils::set_refresh_panels_error(false);
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    assert_eq!(
        app.status,
        Status::CommandOutput("ffm-mcp: 4 added".to_owned())
    );
}
