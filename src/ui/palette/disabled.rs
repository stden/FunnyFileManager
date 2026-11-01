//! Inactive controls remain readable and lose their actionable emphasis.

use ratatui::style::{Color, Modifier, Style};

use super::{BACKGROUND, DIALOG_BG, DIALOG_FG, DISABLED_FG, FIELD_BG, HIGHLIGHT_BG};

const ON_LIGHT: Color = Color::Rgb(72, 72, 72);

pub(in crate::ui) fn disabled_style(base: Style) -> Style {
    let background = base.bg.unwrap_or(BACKGROUND);
    let foreground = if [DIALOG_BG, HIGHLIGHT_BG].contains(&background) {
        ON_LIGHT
    } else if background == FIELD_BG {
        DIALOG_FG
    } else {
        DISABLED_FG
    };
    base.fg(foreground)
        .remove_modifier(Modifier::BOLD | Modifier::UNDERLINED | Modifier::DIM)
        .add_modifier(Modifier::ITALIC)
}
