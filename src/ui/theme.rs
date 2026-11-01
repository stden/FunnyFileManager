//! Purpose: The Colors and Highlighting settings applied over a drawn frame:
//! Far's blue ground swapped for black or grey, and the entry colours
//! flattened when highlighting is off.

use ratatui::Frame;

use super::{palette, FAR_BLUE};
use crate::models::App;

pub(super) fn apply_theme(f: &mut Frame<'_>, app: &App) {
    let background = match app.settings.get(crate::menu::SettingsPage::Colors) {
        "black" => palette::BLACK_BACKGROUND,
        "gray" => palette::GRAY_BACKGROUND,
        _ => FAR_BLUE,
    };
    let highlighting = app
        .settings
        .enabled(crate::menu::SettingsPage::Highlighting);
    if background != FAR_BLUE || !highlighting {
        for cell in &mut f.buffer_mut().content {
            let panel_ground = cell.bg == FAR_BLUE;
            if panel_ground {
                cell.set_bg(background);
            }
            if !highlighting
                && panel_ground
                && matches!(
                    cell.fg,
                    palette::EXECUTABLE
                        | palette::HIDDEN
                        | palette::DIRECTORY
                        | palette::DIRTY_REPO
                )
            {
                cell.set_fg(palette::FILE);
            }
        }
    }
}
