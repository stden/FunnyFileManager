//! Purpose: Renders the dual-panel terminal UI, status bars, and core layout.
//!
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::models::{App, PanelKind, PanelSide, PanelsShown, ViewMode};
#[cfg(test)]
use crate::models::{Overlay, Viewer};

mod apply;
mod archive_commands;
mod archive_prompt;
mod associations;
mod attributes;
mod cells;
mod code;
mod command_line;
mod command_prompt;
mod completion;
mod console;
mod describe;
mod dialog;
mod drives;
mod editor_lsp;
mod editor_selection;
mod extract_prompt;
mod filter;
mod findfile;
mod findfolder;
mod fkey_bar;
mod folder_history;
mod history_list;
mod hotplug;
#[cfg(feature = "keepass")]
mod kdbx;
mod layout;
mod overlays;
mod palette;
mod panel;
mod path_env;
mod quickview;
mod screens;
mod settings;
mod shortcuts;
mod sortmenu;
mod stub;
mod tasks;
mod tree;
mod usermenu;
mod view_history;
mod viewer;
mod wipe;
mod wsl;
pub(crate) use wsl::wsl_button_at;
mod disk_cleanup;

use overlays::render_overlays;

mod menu;
#[cfg(test)]
pub(crate) use menu::checked_entry;
pub use menu::{heading_at, menu_row_at};
mod menu_dialog;
pub(crate) use menu_dialog::row_at as choice_row_at;
pub(crate) use panel::stripe_at;
#[cfg(test)]
use panel::{
    abbreviate_bytes, clip_name, name_field, pad_cells, panel_layout, size_field, split_extension,
};
use panel::{panel_footer, render_panel};
pub(crate) use sortmenu::row_at as sort_row_at;

#[cfg(test)]
pub(crate) use cells::UNPRINTABLE;
pub(crate) use cells::{cells_before, fit_name, printable_name, Cells};
use cells::{head_cells, screen_u16, skip_cells, tail_cells, take_cells};
use command_line::render_command_line;
#[cfg(test)]
use command_prompt::prompt_spans;
use completion::render_completion_menu;
use fkey_bar::render_command_bar;
pub use fkey_bar::FKEY_COUNT;
#[cfg(test)]
use fkey_bar::FKEY_LABELS;
pub use layout::{centered_rect, panel_page_size, FALLBACK_TERMINAL_HEIGHT};
use layout::{
    COMMAND_LINE_HEIGHT, FKEY_BAR_HEIGHT, MENU_BAR_HEIGHT, PANEL_BORDERS, PANEL_COLUMN_HEADER,
    PANEL_COLUMN_RULE, PANEL_INFO_ROW, PANEL_NON_ENTRY_ROWS,
};

/// The panel ground. Kept as a name because it is referenced all over the
/// interface; the value itself lives in [`palette`] with the rest of the scheme.
pub(crate) const FAR_BLUE: Color = palette::BACKGROUND;

/// Base frame used throughout the interface. Far Manager's panels and dialogs
/// use double-line box drawing, so every framed widget starts here instead of
/// selecting its border independently.
pub(super) fn far_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
}

/// Draw the entire user interface: two file panels, status bar, and overlays.
pub fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    console::render_backdrop(f, area);
    if app.view == ViewMode::Console {
        console::render_view(f, app, area);
    } else {
        render_panel_view(f, app, area);
    }
    apply_theme(f, app);
    if app.view == ViewMode::Panels {
        menu::render_drop_down(
            f,
            app,
            Rect {
                height: area.height.min(MENU_BAR_HEIGHT),
                ..area
            },
        );
    }
    render_overlays(f, app);
}

fn apply_theme(f: &mut Frame, app: &App) {
    let background = match app.settings.get(crate::menu::SettingsPage::Colors) {
        "black" => Color::Black,
        "gray" => Color::DarkGray,
        _ => FAR_BLUE,
    };
    let highlighting = app
        .settings
        .enabled(crate::menu::SettingsPage::Highlighting);
    if background != FAR_BLUE || !highlighting {
        for cell in &mut f.buffer_mut().content {
            if cell.bg == FAR_BLUE {
                cell.set_bg(background);
            }
            if !highlighting
                && matches!(
                    cell.fg,
                    palette::EXECUTABLE | palette::HIDDEN | palette::DIRECTORY
                )
            {
                cell.set_fg(palette::FILE);
            }
        }
    }
}

fn render_panel_view(f: &mut Frame, app: &mut App, area: Rect) {
    // No transcript layer here. It used to be painted underneath and "covered
    // by the panels", but covered was never true: a ratatui widget sets
    // symbols only where its text reaches, so the menu bar's 44 cells of
    // headings left the tail of every longer transcript line on screen — and
    // repainted it from the log on every frame, so it never went away. The
    // transcript belongs to the console screen, which `console::render_view`
    // still paints; `render_backdrop` owns the ground here.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(MENU_BAR_HEIGHT),
            Constraint::Min(0),
            Constraint::Length(COMMAND_LINE_HEIGHT),
            Constraint::Length(FKEY_BAR_HEIGHT),
        ])
        .split(area);
    app.dialog_mouse.screen = area;
    menu::render_bar(f, app, chunks[0]);
    render_both_panels(f, app, chunks[1]);
    render_command_line(f, app, chunks[2]);
    render_completion_menu(f, app, chunks[2]);
    render_command_bar(f, chunks[3]);
}

fn render_both_panels(f: &mut Frame, app: &mut App, area: Rect) {
    // Only the active panel replaces its selected-entry footer with status.
    let footer = panel_footer(app);
    let (left, right) = panel_areas(app, area);
    if let Some(area) = left {
        render_one_panel(f, app, PanelSide::Left, area, footer.as_deref());
    }
    if let Some(area) = right {
        render_one_panel(f, app, PanelSide::Right, area, footer.as_deref());
    }
}

/// Where each panel goes, or `None` for one that is off screen.
///
/// A hidden panel (`Ctrl+F1` / `Ctrl+F2`) gives its half to the other, and a
/// fullscreen view mode on the active panel takes the whole width — Far's
/// Detailed columns do not fit in half a screen, which is why Far marks them
/// fullscreen in the first place.
pub(crate) fn panel_areas(app: &App, area: Rect) -> (Option<Rect>, Option<Rect>) {
    // `mouse::hit` reads the same answer, so a click lands where this drew.
    match app.panels_on_screen() {
        PanelsShown::LeftOnly => (Some(area), None),
        PanelsShown::RightOnly => (None, Some(area)),
        PanelsShown::Both => {
            let halves = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            (Some(halves[0]), Some(halves[1]))
        }
    }
}

/// One panel, drawn as whatever kind it is: a file list, a folder tree, or a
/// preview of the other panel's cursor.
fn render_one_panel(
    f: &mut Frame,
    app: &mut App,
    side: PanelSide,
    area: Rect,
    footer: Option<&str>,
) {
    let is_active = app.active_panel == side;
    let panel = app.panel_mut(side);
    match panel.kind {
        PanelKind::Files => render_panel(f, panel, area, is_active, footer.filter(|_| is_active)),
        PanelKind::Tree(_) => tree::render(f, panel, area, is_active),
        PanelKind::QuickView(_) => quickview::render(f, panel, area, is_active),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_console;
#[cfg(test)]
mod tests_delete_confirm_paths;
#[cfg(test)]
mod tests_editor_confirm_exit;
#[cfg(test)]
mod tests_overlay_dialogs;
#[cfg(test)]
mod tests_overlay_render_gaps;
#[cfg(test)]
mod tests_render_widths;
#[cfg(test)]
mod tests_stub_dialogs;
