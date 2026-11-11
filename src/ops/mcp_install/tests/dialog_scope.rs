//! Purpose: Opening the dialog and switching its scope.

use super::*;

#[test]
fn a_new_app_looks_at_no_machine_at_all() {
    let app = App::with_panels(
        Panel::new(std::env::temp_dir()).unwrap(),
        Panel::new(std::env::temp_dir()).unwrap(),
    );
    assert_eq!(app.live.mcp_host.home, None);
    assert_eq!(app.live.mcp_host.search_path.len(), 0);
    assert_eq!(app.live.mcp_host.exe_dir, None);
    assert_eq!(app.live.mcp_host.env.len(), 0);
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
