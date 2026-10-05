//! Purpose: The Colors and Highlighting settings applied over a drawn frame:
//! Far's blue ground swapped for black or grey, and the entry colours
//! flattened when highlighting is off.

use ratatui::{style::Color, Frame};

use super::{palette, FAR_BLUE};
use crate::models::App;

pub(super) fn apply_theme(f: &mut Frame<'_>, app: &App) {
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
