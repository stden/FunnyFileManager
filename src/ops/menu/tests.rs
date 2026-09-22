//! Exercise complete menu workflows against isolated files and configuration.

use super::*;
use crate::{
    menu::{Heading, Menu, SettingsPage},
    models::{Panel, PanelSide, PanelsShown, ViewMode},
    status::Status,
};
use std::fs;
use tempfile::TempDir;

fn fixture() -> (TempDir, App) {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("folder")).unwrap();
    fs::write(temp.path().join("alpha.txt"), "alpha\ttext").unwrap();
    fs::write(temp.path().join("beta.bin"), "beta").unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    app.settings =
        crate::models::settings::Settings::with_file(temp.path().join("config/settings.json"));
    select(&mut app, "alpha.txt");
    (temp, app)
}

fn select(app: &mut App, name: &str) {
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap();
    app.left_panel.select(Some(index));
}

fn rows(app: &App) -> &[MenuChoice] {
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("expected menu dialog")
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("expected choices")
    };
    rows
}

#[test]
fn settings_rows_open_apply_and_persist_their_documented_defaults() {
    let (_temp, mut app) = fixture();
    for entry in Menu::open(Heading::Options).entries() {
        let Some(MenuCommand::Settings(page)) = entry.menu_command() else {
            continue;
        };
        activate(&mut app, *entry);
        assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
        let value = crate::models::settings::specification(page).1;
        submit(&mut app, MenuCommand::Settings(page), value).unwrap();
        assert_eq!(app.settings.get(page), value);
        assert!(!app.overlay.is_active());
    }
    execute_result(&mut app, MenuCommand::SaveSetup).unwrap();
    assert!(app.settings.persistence.file().unwrap().is_file());
}

#[test]
fn file_and_archive_rows_open_their_own_prompts() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::FindFile).unwrap();
    assert!(
        matches!(app.overlay, Overlay::FindFile(_)),
        "Find file has its own overlay"
    );

    for command in [MenuCommand::FindFolder, MenuCommand::Filter] {
        execute_result(&mut app, command).unwrap();
        let Overlay::MenuDialog(dialog) = &app.overlay else {
            panic!("prompt")
        };
        let MenuDialog::Prompt {
            command: actual, ..
        } = dialog.as_ref()
        else {
            panic!("prompt")
        };
        assert_eq!(*actual, command);
    }
}

/// Alt+F6 and the Files menu row are one path: the Link name prompt, which
/// asks the kind as well as the name, not the generic text prompt.
#[test]
fn the_link_row_opens_the_link_name_prompt() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Link).unwrap();
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the Link name prompt")
    };
    assert_eq!(prompt.purpose(), crate::models::NamePurpose::Link);
    assert!(
        submit(&mut app, MenuCommand::Link, "linked.txt").is_err(),
        "Link has no text prompt to submit"
    );
}

/// The Describe row has a dialog of its own rather than the generic prompt:
/// one per marked entry, opening on the current text. The menu hands over to
/// it, and `submit` has no text prompt to answer.
#[test]
fn the_describe_row_opens_its_own_dialog_or_says_there_is_nothing() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Describe).unwrap();
    let Overlay::Describe(prompt) = &app.overlay else {
        panic!("the describe dialog is open")
    };
    assert_eq!(prompt.name(), "alpha.txt");
    assert!(submit(&mut app, MenuCommand::Describe, "text").is_err());

    app.close_overlay();
    app.left_panel.select(None);
    execute_result(&mut app, MenuCommand::Describe).unwrap();
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status,
        Status::CommandOutput("Nothing to describe".to_owned())
    );
}

#[test]
fn search_results_open_files_and_navigate_to_folders() {
    let (temp, mut app) = fixture();
    submit(&mut app, MenuCommand::FindFile, "*.txt").unwrap();
    assert_eq!(rows(&app).len(), 1);
    let target = rows(&app)[0].target.clone();
    choose(&mut app, target).unwrap();
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    assert_eq!(app.viewed_files, vec![temp.path().join("alpha.txt")]);
    submit(&mut app, MenuCommand::FindFolder, "fold*").unwrap();
    let target = rows(&app)[0].target.clone();
    choose(&mut app, target).unwrap();
    assert_eq!(app.left_panel.path, temp.path().join("folder"));
    execute_result(&mut app, MenuCommand::FolderHistory).unwrap();
    assert_eq!(rows(&app).len(), 1);
    execute_result(&mut app, MenuCommand::ViewHistory).unwrap();
    assert_eq!(rows(&app).len(), 1);
}

#[test]
fn folder_search_respects_depth_and_entry_limits() {
    let (temp, mut app) = fixture();
    fs::create_dir_all(temp.path().join("folder/deep/deeper")).unwrap();
    app.settings.set(SettingsPage::Tree, "1").unwrap();
    submit(&mut app, MenuCommand::FindFolder, "*").unwrap();
    assert_eq!(rows(&app).len(), 2);
    app.settings.set(SettingsPage::System, "1").unwrap();
    submit(&mut app, MenuCommand::FindFolder, "*").unwrap();
    assert!(rows(&app).len() <= 1);
}

#[test]
fn the_tree_row_turns_the_active_panel_into_a_folder_tree_and_back() {
    let (temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Tree).unwrap();
    assert!(app.left_panel.kind.is_tree());
    assert!(
        !matches!(app.overlay, Overlay::MenuDialog(_)),
        "a panel, not a Find folder list"
    );
    let tree = crate::ops::tree::tree_mut(&mut app).unwrap();
    assert_eq!(
        tree.selected(),
        Some(temp.path().canonicalize().unwrap().as_path()),
        "opened on the panel's folder"
    );
    execute_result(&mut app, MenuCommand::Tree).unwrap();
    assert_eq!(app.left_panel.kind, crate::models::PanelKind::Files);
}

#[test]
fn filter_preserves_directories_and_can_be_cleared() {
    let (_temp, mut app) = fixture();
    submit(&mut app, MenuCommand::Filter, "*.txt").unwrap();
    assert!(app
        .left_panel
        .entries
        .iter()
        .all(|e| e.kind.is_dir() || e.name == "alpha.txt"));
    submit(&mut app, MenuCommand::Filter, "").unwrap();
    assert!(app.left_panel.entries.iter().any(|e| e.name == "beta.bin"));
    let before = app.left_panel.entries.clone();
    app.left_panel.path.push("missing");
    assert!(submit(&mut app, MenuCommand::Filter, "*.none").is_err());
    assert!(app.left_panel.options.filter.is_empty());
    assert_eq!(app.left_panel.entries.len(), before.len());
}

#[test]
fn panel_on_off_and_name_commands_toggle_only_the_requested_panel() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::TogglePanel(PanelSide::Right)).unwrap();
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    execute_result(&mut app, MenuCommand::TogglePanel(PanelSide::Left)).unwrap();
    assert_eq!(
        app.panels_shown,
        PanelsShown::LeftOnly,
        "the last panel stays"
    );
    execute_result(&mut app, MenuCommand::TogglePanel(PanelSide::Right)).unwrap();
    assert_eq!(app.panels_shown, PanelsShown::Both);
    execute_result(&mut app, MenuCommand::LongNames).unwrap();
    assert!(app.left_panel.options.short_names);
    assert!(!app.right_panel.options.short_names);
}

#[test]
fn histories_screens_and_shortcuts_offer_real_targets() {
    let (temp, mut app) = fixture();
    app.history.record(":pwd");
    execute_result(&mut app, MenuCommand::History).unwrap();
    assert_eq!(rows(&app)[0].label, ":pwd");
    let target = rows(&app)[0].target.clone();
    choose(&mut app, target).unwrap();
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
    execute_result(&mut app, MenuCommand::Screens).unwrap();
    let target = rows(&app)[1].target.clone();
    choose(&mut app, target).unwrap();
    assert_eq!(app.view, ViewMode::Console);
    app.bookmarks.set(
        crate::models::BookmarkSlot::new(2).unwrap(),
        temp.path().to_owned(),
    );
    execute_result(&mut app, MenuCommand::Shortcuts).unwrap();
    assert!(rows(&app)[0].label.starts_with("2:"));
    choose(&mut app, ChoiceTarget::Close).unwrap();
    choose(&mut app, ChoiceTarget::Menu(MenuCommand::Sort)).unwrap();
    assert!(matches!(&app.overlay, Overlay::SortModes(_)));
}

/// The menu row and `Ctrl+Q` share one path: the passive panel becomes a
/// preview of the active cursor, and the row toggles it back off.
#[test]
fn quick_view_previews_the_cursor_on_the_passive_panel_and_toggles_off() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::QuickView).unwrap();
    assert!(!app.overlay.is_active(), "a panel kind, not an overlay");
    assert_eq!(app.left_panel.kind, crate::models::PanelKind::Files);
    let crate::models::PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("expected a quick view: {:?}", app.right_panel.kind);
    };
    let crate::models::Preview::Text(text) = &view.preview else {
        panic!("expected text: {:?}", view.preview);
    };
    assert_eq!(text.lines, vec!["alpha\ttext"]);
    execute_result(&mut app, MenuCommand::QuickView).unwrap();
    assert_eq!(app.right_panel.kind, crate::models::PanelKind::Files);
}

/// Wipe is the one Files row without a text prompt: it asks the way F8 does,
/// with its own confirmation, and `ops::wipe` owns what a Yes then does.
#[test]
fn wipe_asks_with_its_own_confirmation_and_takes_no_text() {
    let (temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Wipe).unwrap();
    let Overlay::WipeConfirm(paths) = &app.overlay else {
        panic!("wipe confirmation")
    };
    assert_eq!(paths, &[temp.path().join("alpha.txt")]);
    assert!(submit(&mut app, MenuCommand::Wipe, "WIPE").is_err());
    assert!(temp.path().join("alpha.txt").is_file());
}

/// Ctrl+G is the one Files row with its own dialog: the `!` substitutions
/// and the per-entry runs live behind `Overlay::ApplyCommand`, so the generic
/// text prompt has nothing to submit for it.
#[test]
fn the_apply_command_row_opens_its_own_dialog() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::ApplyCommand).unwrap();
    assert!(matches!(app.overlay, Overlay::ApplyCommand(_)));
    assert!(submit(&mut app, MenuCommand::ApplyCommand, "echo").is_err());
}

#[test]
fn task_and_device_menus_show_host_information() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Tasks).unwrap();
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    execute_result(&mut app, MenuCommand::Devices).unwrap();
    assert!(!rows(&app).is_empty());
}

#[test]
fn command_failures_are_reported_without_terminating_the_application() {
    let (_temp, mut app) = fixture();
    app.settings.persistence = crate::history::Persistence::SessionOnly;
    execute(&mut app, MenuCommand::SaveSetup);
    assert!(matches!(app.status, Status::Error(_)));
    assert!(app.lifecycle.is_running());
    assert!(submit(&mut app, MenuCommand::Sort, "unsupported").is_err());
}

/// The three archive rows open Far's own dialogs, which have controls of
/// their own rather than the one-line text prompt.
#[test]
fn archive_rows_open_their_own_dialogs_and_have_no_text_prompt() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Archive).unwrap();
    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
    app.close_overlay();
    execute_result(&mut app, MenuCommand::Extract).unwrap();
    assert!(matches!(app.overlay, Overlay::ExtractPrompt(_)));
    app.close_overlay();
    execute_result(&mut app, MenuCommand::ArchiveCommands).unwrap();
    assert!(matches!(app.overlay, Overlay::ArchiveCommands(_)));
    app.close_overlay();
    assert!(submit(&mut app, MenuCommand::Archive, "bundle.tar").is_err());
    assert!(submit(&mut app, MenuCommand::Extract, "unpacked").is_err());
}

#[test]
fn compare_marks_missing_or_different_sized_files_on_both_sides() {
    let (_temp, mut app) = fixture();
    let right = TempDir::new().unwrap();
    fs::write(right.path().join("alpha.txt"), "short").unwrap();
    fs::write(right.path().join("beta.bin"), "beta").unwrap();
    fs::write(right.path().join("right-only"), "new").unwrap();
    app.right_panel = Panel::new(right.path().to_owned()).unwrap();
    execute_result(&mut app, MenuCommand::Compare).unwrap();
    assert!(app.left_panel.marked.contains("alpha.txt"));
    assert!(app.right_panel.marked.contains("alpha.txt"));
    assert!(app.right_panel.marked.contains("right-only"));
    assert!(!app.left_panel.marked.contains("beta.bin"));
    assert!(!app.left_panel.marked.contains("folder"));
}

#[test]
fn file_prompts_reject_empty_or_ambiguous_selections() {
    let (_temp, mut app) = fixture();
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("alpha.txt"));
    app.left_panel
        .marked
        .insert(crate::models::FileName::from("beta.bin"));
    app.left_panel.marked.clear();
    select(&mut app, "alpha.txt");
    assert!(submit(&mut app, MenuCommand::Attributes, "0644").is_err());
}

/// Ctrl+A and the Files-menu row open Far's attributes dialog on the
/// selection, not a text prompt.
#[test]
fn the_attributes_row_opens_the_attributes_dialog() {
    let (_temp, mut app) = fixture();
    execute_result(&mut app, MenuCommand::Attributes).unwrap();
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
    assert!(user::edit(&mut app, MenuCommand::Sort).is_err());
}
