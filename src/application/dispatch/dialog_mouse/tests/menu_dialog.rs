//! Generic F9 dialogs activate the same operations from rendered mouse targets.

use super::*;
use crate::action::ToolCommand;
use crate::menu::SettingsPage;
use crate::models::{ChoiceTarget, MenuChoice, MenuDialog, ViewMode};

fn open_filter(app: &mut App) {
    crate::ops::menu::execute_tool(app, ToolCommand::Filter).expect("the filter prompt");
}

fn choices(app: &mut App, count: usize, selected: usize) {
    let rows = (0..count)
        .map(|index| MenuChoice {
            label: format!("Console choice {index:02}"),
            target: ChoiceTarget::Screen(ViewMode::Console),
        })
        .collect();
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: "Screens".into(),
        rows,
        selected,
    })));
}

#[test]
fn menu_choice_click_activates_visible_row_even_after_scrolling() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();
    choices(&mut app, 40, 39);
    click_text(&mut term, &mut app, "Console choice 39");
    assert_eq!(app.view, ViewMode::Console);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn menu_choice_click_activates_clicked_row_instead_of_old_selection() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();
    choices(&mut app, 2, 1);
    let Overlay::MenuDialog(dialog) = &mut app.overlay else {
        panic!("menu")
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_mut() else {
        panic!("choices")
    };
    rows[0].target = ChoiceTarget::Screen(ViewMode::Panels);
    app.view = ViewMode::Console;
    click_text(&mut term, &mut app, "Console choice 00");
    assert_eq!(app.view, ViewMode::Panels);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn empty_menu_and_menu_chrome_consume_clicks_without_activation() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();
    for count in [0, 2] {
        choices(&mut app, count, 0);
        click_text(&mut term, &mut app, "Screens");
        gesture(
            &mut term,
            &mut app,
            MouseEventKind::Down(MouseButton::Left),
            (0, 0),
        );
        assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
        assert_eq!(app.view, ViewMode::Panels);
        assert!(app.dialog_mouse.grabbed.is_none());
    }
}

#[test]
fn menu_prompt_buttons_apply_and_cancel_when_escape_is_disabled() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();
    app.settings.set(SettingsPage::Dialog, "off").unwrap();
    open_filter(&mut app);
    popup::process_popup_input(&mut app, KeyCode::Char('x').into());
    click_text(&mut term, &mut app, "OK");
    assert_eq!(app.left_panel.options.filter, "x");
    assert!(matches!(app.overlay, Overlay::None));
    open_filter(&mut app);
    click_text(&mut term, &mut app, "Cancel");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.left_panel.options.filter, "x");
}

#[test]
fn only_prompt_variants_expose_apply_and_cancel_button_keys() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    open_filter(&mut app);
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(code(&mut app, 1), KeyCode::F(10));
    assert!(dialog_button_key(&mut app, 2).is_none());
    choices(&mut app, 1, 0);
    assert!(dialog_button_key(&mut app, 0).is_none());
    crate::ops::sortmenu::open(&mut app);
    assert!(dialog_button_key(&mut app, 0).is_none());
}

/// A click on a sort row chooses it, the way Enter would; the rule between
/// the modes and the order switch names no row, so a press there is a grip.
#[test]
fn a_click_on_a_sort_row_chooses_it_and_the_rule_is_a_grip() {
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();

    crate::ops::sortmenu::open(&mut app);
    click_text(&mut term, &mut app, "Creation time");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(
        app.left_panel.sort_mode,
        crate::models::SortMode::CreationTime
    );

    crate::ops::sortmenu::open(&mut app);
    click_text(&mut term, &mut app, "Reverse order");
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(
        app.left_panel.sort_order,
        crate::models::SortOrder::Reversed
    );

    crate::ops::sortmenu::open(&mut app);
    draw(&mut app, &mut term);
    let area = app.dialog_mouse.area.expect("the menu was drawn");
    let rule = (area.x + 2, area.y + 1 + 9);
    gesture(
        &mut term,
        &mut app,
        MouseEventKind::Down(MouseButton::Left),
        rule,
    );
    assert!(matches!(app.overlay, Overlay::SortModes(_)));
    assert_eq!(app.dialog_mouse.grabbed, Some(rule));
}

#[test]
fn clicking_plugin_commands_activates_clicked_row_and_ignores_border_or_unmeasured_area() {
    use super::super::menus::click_plugin_commands;
    let dir = TempDir::new().unwrap();
    let mut app = app_at(&dir);
    let mut term = terminal();

    assert!(!click_plugin_commands(&mut term, &mut app, 5, 5));
    app.set_overlay(Overlay::PluginCommands(
        crate::models::PluginCommands::default(),
    ));
    assert!(!click_plugin_commands(&mut term, &mut app, 5, 5));

    app.dialog_mouse.area = Some(ratatui::layout::Rect::new(10, 10, 20, 20));
    assert!(!click_plugin_commands(&mut term, &mut app, 10, 10));
    assert!(!click_plugin_commands(&mut term, &mut app, 15, 25));
    assert!(click_plugin_commands(&mut term, &mut app, 15, 12));
    assert!(!matches!(app.overlay, Overlay::PluginCommands(_)));
}

#[test]
fn remaining_dialog_buttons_route_to_controls() {
    use crate::models::{
        ArchiveFormat, ArchivePrompt, DiskCleanupDialog, FindFile, OperationFailed, SessionEdit,
    };
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let code = |app: &mut App, btn: usize| dialog_button_key(app, btn).expect("button").code;
    app.set_overlay(Overlay::FindFile(Box::<FindFile>::default()));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    app.set_overlay(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    app.set_overlay(Overlay::DiskCleanup(Box::<DiskCleanupDialog>::default()));
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    app.set_overlay(Overlay::ArchivePrompt(Box::new(ArchivePrompt::new(
        vec!["a.txt".into()],
        "a",
        dir.path(),
        vec![ArchiveFormat::TarGz],
    ))));
    assert_eq!(
        (code(&mut app, 0), code(&mut app, 1)),
        (KeyCode::Enter, KeyCode::Enter)
    );
    let failed = crate::status::FailedEntry::new(dir.path().join("x"), &"denied");
    let stop = OperationFailed::new(crate::progress::Operation::Delete, failed, 0, 1);
    app.set_overlay(Overlay::OperationFailed(Box::new(stop)));
    assert_eq!(code(&mut app, 0), KeyCode::Esc);
}
