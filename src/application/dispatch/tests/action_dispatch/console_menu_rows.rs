//! Purpose: in console mode a menu row or gesture still runs its F9 command and its panel command, over the transcript.

use super::*;
use crate::action::PanelCommand;
use crate::menu::MenuCommand;

#[test]
fn a_menu_row_still_opens_its_dialog_over_the_console() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    app.view = ViewMode::Console;

    dispatch_action(
        &mut terminal,
        &mut app,
        Action::Menu(MenuCommand::Shortcuts),
    );

    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
    assert_eq!(app.view, ViewMode::Console);
}

#[test]
fn a_panel_row_still_changes_the_active_panel_over_the_console() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    app.view = ViewMode::Console;

    dispatch_action(&mut terminal, &mut app, Action::Panel(PanelCommand::Tree));

    assert!(app.get_active_panel_ref().kind.is_tree());
    assert_eq!(app.view, ViewMode::Console);
}

/// The menu's own keys set `menu_key`; the same command arriving without it
/// (as a chord that is not the menu's would) is swallowed over the console.
#[test]
fn a_menu_command_without_its_menu_key_is_swallowed_over_the_console() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    app.view = ViewMode::Console;

    apply_action(
        &mut terminal,
        &mut app,
        Action::Menu(MenuCommand::Shortcuts),
        false,
    );

    assert!(
        !app.is_overlay_active(),
        "the menu command opened a dialog behind the console"
    );
}
