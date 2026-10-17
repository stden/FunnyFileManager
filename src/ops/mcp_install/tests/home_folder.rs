//! Purpose: The current folder at the home folder, and a registration without `claude`.

use super::*;

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
