//! Purpose: Rows that open tools and dialogs, and failures that never end the application.

use super::*;

/// Wipe asks the way F8 does, with its own confirmation, and `ops::wipe`
/// owns what a Yes then does.
#[test]
fn wipe_asks_with_its_own_confirmation() {
    let (temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::Wipe);
    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("wipe confirmation")
    };
    assert_eq!(paths, &[temp.path().join("alpha.txt")]);
    assert!(temp.path().join("alpha.txt").is_file());
}

/// Ctrl+G has a dialog of its own: the `!` substitutions and the per-entry
/// runs live behind `Overlay::ApplyCommand`.
#[test]
fn the_apply_command_row_opens_its_own_dialog() {
    let (_temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::ApplyCommand);
    assert!(matches!(app.overlay, Overlay::ApplyCommand(_)));
}

/// The Task list row opens the live viewer on the installed host, never a
/// second copy of the old report.
#[test]
fn the_task_list_row_opens_the_live_viewer() {
    let (_temp, mut app) = fixture();
    let host = crate::test_support::processes::FixtureHost::new(Vec::new());
    app.task_host.install(std::sync::Arc::new(host));
    execute_tool(&mut app, ToolCommand::Tasks).unwrap();
    assert!(matches!(app.overlay, Overlay::Tasks(_)));
}

/// The F9 row runs the same dialog as `:cleanup`, which measures the rows
/// under the home first, so the home is a temporary one.
#[test]
fn the_disk_cleanup_row_opens_the_dialog() {
    let home = TempDir::new().unwrap();
    let _home = crate::test_support::EnvVarsGuard::set(&[("HOME", home.path())]);
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::DiskCleanup).unwrap();
    assert!(matches!(app.overlay, Overlay::DiskCleanup(_)));
}

#[test]
fn the_devices_row_lists_the_mounted_devices() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Devices).unwrap();
    assert_ne!(rows(&app).len(), 0);
}

#[test]
fn command_failures_are_reported_without_terminating_the_application() {
    let (_temp, mut app) = fixture();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    execute(&mut app, MenuCommand::SaveSetup);
    assert!(matches!(app.status, Status::Error(_)));
    assert!(app.lifecycle.is_running());
}

/// The three archive rows open Far's own dialogs, which have controls of
/// their own rather than the one-line text prompt.
#[test]
fn archive_rows_open_their_own_dialogs() {
    let (_temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::AddToArchive);
    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
    app.close_overlay();
    execute_file(&mut app, FileCommand::Extract);
    assert!(matches!(app.overlay, Overlay::ExtractPrompt(_)));
    app.close_overlay();
    execute_file(&mut app, FileCommand::ArchiveCommands);
    assert!(matches!(app.overlay, Overlay::ArchiveCommands(_)));
}

#[test]
fn compare_marks_missing_or_different_sized_files_on_both_sides() {
    let (_temp, mut app) = fixture();
    let right = TempDir::new().unwrap();
    fs::write(right.path().join("alpha.txt"), "short").unwrap();
    fs::write(right.path().join("beta.bin"), "beta").unwrap();
    fs::write(right.path().join("right-only"), "new").unwrap();
    fs::write(right.path().join("folder"), "").unwrap();
    app.right_panel = Panel::new(right.path().to_owned()).unwrap();
    execute_result(&mut app, MenuCommand::Compare).unwrap();
    assert!(app.left_panel.marked.contains("alpha.txt"));
    assert!(app.right_panel.marked.contains("alpha.txt"));
    assert!(app.right_panel.marked.contains("right-only"));
    assert!(app.right_panel.marked.contains("folder"));
    assert!(!app.left_panel.marked.contains("beta.bin"));
    assert!(!app.left_panel.marked.contains("folder"));
}

/// Ctrl+A and the Files-menu row open Far's attributes dialog on the
/// selection, not a text prompt.
#[test]
fn the_attributes_row_opens_the_attributes_dialog() {
    let (_temp, mut app) = fixture();
    execute_file(&mut app, FileCommand::Attributes);
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("attributes dialog")
    };
    assert!(
        dialog.headline().ends_with("alpha.txt"),
        "{}",
        dialog.headline()
    );
}

#[test]
fn performance_update_packages_and_path_menu_rows_open_their_dialogs() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Performance).unwrap();
    assert!(app.overlay.is_active());
    app.close_overlay();
    execute_result(&mut app, MenuCommand::UpdatePackages).unwrap();
    app.close_overlay();
    drop(execute_result(
        &mut app,
        MenuCommand::Path(crate::path_env::Scope::User),
    ));
}

/// The F9 row lists the connections `databases.yaml` names in the config
/// directory, which is a temporary one.
#[test]
fn the_database_explorer_row_opens_the_connections_of_databases_yaml() {
    let config = TempDir::new().unwrap();
    let _env = crate::test_support::EnvVarsGuard::set(&[(
        crate::test_support::CONFIG_HOME,
        config.path(),
    )]);
    let folder = crate::config::dir().unwrap();
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("databases.yaml"),
        "- name: shop\n  kind: sqlite\n  path: shop.db\n",
    )
    .unwrap();
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::DatabaseExplorer).unwrap();
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("database explorer")
    };
    assert_eq!(explorer.connections().len(), 1);
    assert_eq!(explorer.notice(), None);
}

/// With no `databases.yaml` the explorer still opens, and says where to create it.
#[test]
fn the_database_explorer_row_without_a_config_file_opens_with_a_hint() {
    let config = TempDir::new().unwrap();
    let _env = crate::test_support::EnvVarsGuard::set(&[(
        crate::test_support::CONFIG_HOME,
        config.path(),
    )]);
    let (_temp, mut app) = fixture();
    execute(&mut app, MenuCommand::DatabaseExplorer);
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("database explorer")
    };
    assert!(explorer.connections().is_empty());
    assert!(explorer
        .notice()
        .is_some_and(|text| text.contains("databases.yaml")));
    assert!(matches!(&app.status, Status::Error(text) if text.contains("databases.yaml")));
}

#[test]
fn the_language_server_menu_command_opens_its_editor() {
    let (_temp, mut app) = fixture();
    execute(&mut app, MenuCommand::LanguageServers);
    assert!(matches!(app.overlay, Overlay::LspEditor(_)));
}

/// The user PATH row edits the profile under the home it is given, never the
/// real one.
#[cfg(unix)]
#[test]
fn the_user_path_menu_reads_the_temporary_home_profile() {
    let (temp, mut app) = fixture();
    let _home = crate::test_support::EnvVarGuard::set("HOME", temp.path());
    execute(&mut app, MenuCommand::Path(crate::path_env::Scope::User));
    let Overlay::PathEditor(editor) = &app.overlay else {
        panic!("PATH editor");
    };
    assert!(std::path::Path::new(&editor.snapshot.location).starts_with(temp.path()));
}
