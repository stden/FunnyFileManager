//! Purpose: Rows that change the panels: tree, filter, panel on/off, histories and quick view.

use super::*;

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
