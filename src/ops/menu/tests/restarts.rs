//! Purpose: User menu files and view history outlive a restart, and side menus select their panel.

use super::*;

#[test]
fn edited_command_files_are_persistent_and_available_in_the_user_menu() {
    let (temp, mut app) = fixture();
    for command in [MenuCommand::UserMenu, MenuCommand::Associations] {
        execute_result(&mut app, command).unwrap();
        assert!(matches!(app.overlay, Overlay::Editor(_)));
    }
    fs::write(
        temp.path().join("config/associations.txt"),
        "TXT\techo {}\n",
    )
    .unwrap();
    let associated = user::associated_command(&app).unwrap().unwrap();
    assert!(associated.contains("alpha.txt"));
    select(&mut app, "beta.bin");
    assert!(user::associated_command(&app).unwrap().is_none());
    select(&mut app, "folder");
    assert!(user::associated_command(&app).unwrap().is_none());
    app.left_panel.select(None);
    assert!(user::associated_command(&app).unwrap().is_none());
    app.settings
        .set(SettingsPage::Plugins, "echo plugin")
        .unwrap();
    user::open(&mut app).unwrap();
    assert_eq!(rows(&app)[0].label, "Configured plugin command");
    assert!(user::edit(&mut app, MenuCommand::Compare).is_err());
}

/// Alt+F11 lists the view history ffm persists, so a restart does not empty
/// it: the files viewed or edited in an earlier session come back, newest
/// first.
#[test]
fn view_history_lists_the_files_viewed_before_a_restart() {
    let (temp, mut app) = fixture();
    let data = TempDir::new().unwrap();
    let _xdg = crate::test_support::EnvVarGuard::set("XDG_DATA_HOME", data.path());
    app.view_history = crate::models::ViewHistory::load();
    crate::ops::handle_view(&mut app).unwrap();
    app.view_history.touch(&temp.path().join("beta.bin"));
    app.view_history.save().unwrap();

    let (_temp, mut restarted) = fixture();
    restarted.view_history = crate::models::ViewHistory::load();
    execute_tool(&mut restarted, ToolCommand::ViewHistory).unwrap();

    let labels: Vec<_> = rows(&restarted)
        .iter()
        .map(|row| row.label.clone())
        .collect();
    assert_eq!(
        labels,
        [
            temp.path().join("beta.bin").display().to_string(),
            temp.path().join("alpha.txt").display().to_string(),
        ]
    );
}

/// Choosing a row of the Right menu runs it on the right panel; a hidden
/// panel's row still runs, but the cursor stays on the panel on screen.
#[test]
fn activate_row_selects_the_panel_of_a_left_or_right_row_that_is_on_screen() {
    let (_temp, mut app) = fixture();
    app.active_panel = PanelSide::Left;
    activate_row(&mut app, Menu::open(Heading::Right));
    assert_eq!(app.active_panel, PanelSide::Right);

    app.active_panel = PanelSide::Left;
    app.panels_shown = PanelsShown::LeftOnly;
    activate_row(&mut app, Menu::open(Heading::Right));
    assert_eq!(
        app.active_panel,
        PanelSide::Left,
        "the right panel is hidden"
    );

    activate_row(&mut app, Menu::open(Heading::Files));
    assert_eq!(
        app.active_panel,
        PanelSide::Left,
        "Files rows keep the panel"
    );
}
