//! Unit tests: the four action families the completed F9 menu brought, and
//! the handoff that runs a chosen row once the drop-down has closed.

use super::*;
use crate::action::{FileCommand, PanelCommand, ToolCommand};
use crate::menu::{Heading, Menu, MenuCommand, SettingsPage};
use crate::models::{PanelSide, PanelView, PanelsShown};

/// Whether an overlay is the one a command was meant to open.
type Opens = fn(&Overlay) -> bool;

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

#[cfg(test)]
mod tests_command_dialogs;
#[cfg(test)]
mod tests_menu_rows_and_enter;

#[test]
fn a_view_mode_lands_on_the_active_panel() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    app.active_panel = PanelSide::Right;
    apply_action(
        &mut terminal,
        &mut app,
        Action::Panel(PanelCommand::View(PanelView::Wide)),
    );
    assert_eq!(app.right_panel.view, PanelView::Wide);
    assert_eq!(
        app.left_panel.view,
        PanelView::Full,
        "the other is untouched"
    );
}

#[test]
fn hiding_the_active_panel_makes_the_other_active() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    apply_action(
        &mut terminal,
        &mut app,
        Action::MenuCommand(MenuCommand::TogglePanel(PanelSide::Left)),
    );
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn tree_quick_view_and_long_names_toggle_back_and_forth() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    apply_action(
        &mut terminal,
        &mut app,
        Action::MenuCommand(MenuCommand::Tree),
    );
    assert!(app.get_active_panel_ref().kind.is_tree());
    // Far's Ctrl+Q previews on the passive panel; the cursor stays put.
    apply_action(
        &mut terminal,
        &mut app,
        Action::Panel(PanelCommand::QuickView),
    );
    assert!(app.get_inactive_panel_ref().kind.is_quick_view());
    assert!(app.get_active_panel_ref().kind.is_tree());
    apply_action(
        &mut terminal,
        &mut app,
        Action::Panel(PanelCommand::QuickView),
    );
    assert_eq!(app.get_inactive_panel_ref().kind, PanelKind::Files);

    apply_action(
        &mut terminal,
        &mut app,
        Action::MenuCommand(MenuCommand::LongNames),
    );
    assert!(app.get_active_panel_ref().options.short_names);
    assert_eq!(app.status_text(), "Short names");
}

#[test]
fn a_sort_key_sets_the_mode_without_a_dialog() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    apply_action(
        &mut terminal,
        &mut app,
        Action::Panel(PanelCommand::Sort(models::SortMode::Size)),
    );
    assert_eq!(app.get_active_panel_ref().sort_mode, models::SortMode::Size);
    assert!(!app.overlay.is_active());
}

#[test]
fn a_panel_view_action_sets_the_active_listing() {
    let mut app = App::new().expect("test app");
    apply_action(
        &mut terminal(),
        &mut app,
        Action::PanelView(PanelView::Wide),
    );
    assert_eq!(app.get_active_panel_ref().view, PanelView::Wide);
}

#[test]
fn a_tree_panel_consumes_movement_keys() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    apply_action(
        &mut terminal,
        &mut app,
        Action::MenuCommand(MenuCommand::Tree),
    );
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Down));
    assert!(app.get_active_panel_ref().kind.is_tree());
}

#[test]
fn a_quick_view_panel_consumes_its_own_keys() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    apply_action(
        &mut terminal,
        &mut app,
        Action::Panel(PanelCommand::QuickView),
    );
    apply_action(&mut terminal, &mut app, Action::SwitchPanel);
    assert!(app.get_active_panel_ref().kind.is_quick_view());
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Down));
}

#[test]
fn plain_dispatch_still_names_grouped_actions() {
    let mut app = App::new().expect("test app");
    let mut terminal = terminal();
    apply_plain_action(&mut terminal, &mut app, Action::PanelView(PanelView::Brief));
    assert_eq!(app.get_active_panel_ref().view, PanelView::Brief);
    apply_plain_action(&mut terminal, &mut app, Action::File(FileCommand::Link));
    apply_plain_action(&mut terminal, &mut app, Action::Menu(MenuCommand::FindFile));
}

#[test]
fn long_names_toggle_through_the_panel_command() {
    let mut app = App::new().expect("test app");
    apply_action(
        &mut terminal(),
        &mut app,
        Action::Panel(PanelCommand::LongNames),
    );
    assert!(app.get_active_panel_ref().options.short_names);
}

#[test]
fn hiding_a_panel_through_the_panel_command_matches_the_menu() {
    let mut app = App::new().expect("test app");
    app.active_panel = PanelSide::Left;
    apply_action(
        &mut terminal(),
        &mut app,
        Action::Panel(PanelCommand::TogglePanel(PanelSide::Left)),
    );
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
}
