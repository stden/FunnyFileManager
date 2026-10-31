//! Purpose: Renders the dual-panel terminal UI, status bars, and core layout.
//!
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Padding},
    Frame,
};

#[cfg(test)]
use crate::models::Viewer;
use crate::models::{App, Overlay, ViewMode};

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
mod editor_brackets;
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
mod lsp_editor;
mod mcp_install;
mod overlays;
mod palette;
mod panel;
mod panel_view;
mod path_env;
mod plugin_commands;
mod progress;
mod prompts;
mod quickview;
mod remote;
mod remote_dialog;
mod screens;
mod session_edit;
mod sessions;
mod settings;
mod shortcuts;
mod sortmenu;
mod stub;
mod tasks;
mod theme;
pub(crate) use tasks::{task_click_at, TaskClick};
mod tree;
mod usermenu;
mod view_history;
mod viewer;
mod wipe;
mod wsl;
pub(crate) use wsl::wsl_button_at;
mod disk_cleanup;

use overlays::render_overlays;
pub(crate) use panel_view::panel_areas;
use panel_view::render_panel_view;
use theme::apply_theme;

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
pub(crate) use sortmenu::row_at as sort_row_at;

#[cfg(test)]
pub(crate) use cells::UNPRINTABLE;
use cells::{
    expand_tabs, fit_path, fit_quoting_path, head_cells, screen_u16, skip_cells, tail_cells,
    take_cells, EditorLine,
};
pub(crate) use cells::{fit_name, printable_name, Cells};
use command_line::render_command_line;
#[cfg(test)]
use command_prompt::prompt_spans;
use completion::render_completion_menu;
pub use fkey_bar::FKEY_COUNT;
pub use layout::{centered_rect, panel_page_size, FALLBACK_TERMINAL_HEIGHT};
use layout::{
    MENU_BAR_HEIGHT, PANEL_BORDERS, PANEL_COLUMN_HEADER, PANEL_COLUMN_RULE, PANEL_INFO_ROW,
    PANEL_NON_ENTRY_ROWS,
};
pub use progress::progress_dialog;

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

pub(super) fn dialog_frame<'a>(title: &'a str, footer: &'a str, ground: Style) -> Block<'a> {
    far_block()
        .title(Line::from(title).centered())
        .title_bottom(Line::from(footer).centered())
        .border_style(Style::default().fg(palette::DRIVE_FG))
        .padding(Padding::horizontal(1))
        .style(ground)
}

pub(super) fn entry_label(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || printable_name(&path.display().to_string()).into_owned(),
        |name| printable_name(&name.to_string_lossy()).into_owned(),
    )
}

pub(super) fn active_title_style(is_active: bool) -> Style {
    if is_active {
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG)
    } else {
        Style::default().fg(palette::FRAME).bg(FAR_BLUE)
    }
}

pub(super) const fn scroll_percent(top: usize, rows: usize, total: usize) -> usize {
    if top.saturating_add(rows) >= total {
        100
    } else {
        top * 100 / total
    }
}

/// Draw the entire user interface: two file panels, status bar, and overlays.
pub fn ui(f: &mut Frame<'_>, app: &mut App) {
    let area = f.area();
    app.dialog_mouse.screen = area;
    if !matches!(app.overlay, Overlay::Viewer(_) | Overlay::Editor(_)) {
        f.render_widget(Clear, area);
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
    }
    render_overlays(f, app);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_delete_confirm_kinds;
#[cfg(test)]
mod tests_delete_confirm_paths;
#[cfg(test)]
mod tests_editor_tabs;
#[cfg(test)]
mod tests_stub_dialogs;
