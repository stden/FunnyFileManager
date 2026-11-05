//! Purpose: the frame and background styles every panel, dialog and menu draws with.
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Padding},
};

use super::palette;

/// The panel ground. Kept as a name because it is referenced all over the
/// interface; the value itself lives in [`palette`] with the rest of the scheme.
pub(super) const FAR_BLUE: Color = palette::BACKGROUND;

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

pub(super) fn active_title_style(is_active: bool) -> Style {
    if is_active {
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG)
    } else {
        Style::default().fg(palette::FRAME).bg(FAR_BLUE)
    }
}
