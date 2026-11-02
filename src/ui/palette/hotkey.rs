//! Keyboard hints stay visible on both light dialogs and dark panels.

use ratatui::style::{Modifier, Style};

use super::{BACKGROUND, DIALOG_BG, DIALOG_FG, FIELD_BG, HIGHLIGHT_BG, HOTKEY};

pub(in crate::ui) fn hotkey_style(base: Style) -> Style {
    let background = base.bg.unwrap_or(BACKGROUND);
    let foreground = if [DIALOG_BG, FIELD_BG, HIGHLIGHT_BG].contains(&background) {
        DIALOG_FG
    } else {
        HOTKEY
    };
    base.fg(foreground)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

#[cfg(test)]
mod tests;
