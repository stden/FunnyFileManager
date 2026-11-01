//! Purpose: cursor movement, paging, quick search, and opening the entry under the cursor.

use std::io::Write;

use ratatui::{backend::Backend, Terminal};

use crate::models::{self, App};
use crate::{ops, status};

use super::{filesystem_root, panel_mouse, report};

pub(super) fn apply_navigation<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    command: crate::action::Navigation,
) {
    use crate::action::Navigation;
    match command {
        Navigation::NavDown => app.get_active_panel().next(),
        Navigation::NavUp => app.get_active_panel().previous(),
        Navigation::Open => open_panel_entry(terminal, app),
        Navigation::SwitchPanel => ops::panels::switch_panel(app),
        Navigation::First => app.get_active_panel().select(Some(0)),
        Navigation::Last => select_last(app),
        Navigation::PageUp => {
            let page = panel_mouse::page(app, panel_page(terminal));
            app.get_active_panel().page_up(page);
        }
        Navigation::PageDown => {
            let page = panel_mouse::page(app, panel_page(terminal));
            app.get_active_panel().page_down(page);
        }
        Navigation::GoRoot => go_root(app),
        Navigation::SwapPanels => ops::panels::swap_panels(app),
    }
}

fn select_last(app: &mut App) {
    let panel = app.get_active_panel();
    if !panel.entries.is_empty() {
        panel.select(Some(panel.entries.len() - 1));
    }
}

fn go_root(app: &mut App) {
    let root = filesystem_root(&app.get_active_panel_ref().path);
    report(app, |app| app.get_active_panel().navigate_to(root));
}

pub(super) fn shorten_quick_search(app: &mut App) {
    if let Some(query) = app.quick_search.backspace().map(str::to_owned) {
        app.get_active_panel().find_first_matching(&query);
    }
}

/// Extend the query and restart it when Far-style search no longer matches.
pub(super) fn quick_search(app: &mut App, character: char) {
    // Query rules belong to QuickSearch; the panel owns matching its entries.
    let query = app.quick_search.push(character).to_owned();
    if !app.get_active_panel().find_first_matching(&query) && app.quick_search.should_restart() {
        let restarted = app.quick_search.restart_with(character).to_owned();
        app.get_active_panel().find_first_matching(&restarted);
    }
}

/// `Left` / `Right`. In a multi-column view they step the cursor a column,
/// as Far's do — Brief is unusable without it — and stop on the first or last
/// entry. In a one-column view this program's arrows choose the panel.
pub(super) fn select_panel_or_column<B: Backend>(
    terminal: &Terminal<B>,
    app: &mut App,
    side: models::PanelSide,
) {
    if panel_mouse::columns(app.get_active_panel_ref()) == 1 {
        ops::panels::select(app, side);
        return;
    }
    let column = panel_page(terminal);
    let panel = app.get_active_panel();
    match side {
        models::PanelSide::Left => panel.page_up(column),
        models::PanelSide::Right => panel.page_down(column),
    }
}

/// Page by the visible panel height, using the same geometry as rendering.
fn panel_page<B: Backend>(terminal: &Terminal<B>) -> usize {
    let height = terminal
        .size()
        .map_or(crate::ui::FALLBACK_TERMINAL_HEIGHT, |size| size.height);
    crate::ui::panel_page_size(height)
}

fn open_panel_entry<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) {
    // Vault interpretation takes priority over executable flags on FAT mounts.
    if ops::vault::open_vault_if_kdbx(app) {
        return;
    }
    match ops::menu::user::associated_command(app) {
        Ok(Some(command)) => {
            report(app, |app| ops::execute_command(app, &command));
            return;
        }
        Err(error) => {
            app.set_status(status::Status::Error(error.to_string()));
            return;
        }
        Ok(None) => {}
    }
    let executable = app
        .get_active_panel()
        .get_selected_entry()
        .is_some_and(|entry| entry.executable && !entry.kind.is_dir());
    if executable {
        report(app, |app| ops::program::launch(terminal, app));
    } else if app
        .get_active_panel()
        .get_selected_entry()
        .is_some_and(|entry| !entry.kind.is_dir())
    {
        report(app, ops::system_open::selected);
    } else {
        report(app, |app| app.get_active_panel().enter());
    }
}
