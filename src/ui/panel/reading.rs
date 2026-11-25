//! Purpose: Far-style ` Reading: N files ` banner drawn across a panel's top border.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::super::{palette, FAR_BLUE, MENU_BAR_HEIGHT, PANEL_BORDERS};
use crate::progress::{DirectoryRead, PanelHeader};

/// The 1-row top-border strip for `read`, covering the interior of the reading
/// panel's top frame so any wider previous path title is replaced cleanly.
#[must_use]
pub fn reading_banner(screen: Rect, read: &DirectoryRead) -> Buffer {
    let Some(area) = banner_area(screen, read.header) else {
        return Buffer::empty(Rect::default());
    };
    let inner = usize::from(area.width);
    let raw = format!(" Reading: {} files ", read.files);
    let label: String = raw.chars().take(inner).collect();
    let left = inner.saturating_sub(label.len()) / 2;
    let right = inner.saturating_sub(label.len()).saturating_sub(left);
    let rule_style = Style::default()
        .fg(palette::FRAME)
        .bg(FAR_BLUE)
        .add_modifier(Modifier::BOLD);
    let badge_style = Style::default().fg(palette::MENU_FG).bg(palette::MENU_BG);
    let line = Line::from(vec![
        Span::styled("═".repeat(left), rule_style),
        Span::styled(label, badge_style),
        Span::styled("═".repeat(right), rule_style),
    ]);
    let mut buffer = Buffer::empty(area);
    line.render(area, &mut buffer);
    buffer
}

fn banner_area(screen: Rect, header: Option<PanelHeader>) -> Option<Rect> {
    if screen.width == 0 || screen.height == 0 {
        return None;
    }
    let (x, y, width) = match header {
        Some(header) => (
            header.x.clamp(screen.x, screen.right() - 1),
            header.y.clamp(screen.y, screen.bottom() - 1),
            header.width,
        ),
        None => (
            screen.x,
            screen
                .y
                .saturating_add(MENU_BAR_HEIGHT)
                .min(screen.bottom() - 1),
            (screen.width / 2).max(1),
        ),
    };
    let clipped_width = width.min(screen.right() - x);
    let inner_width = clipped_width.saturating_sub(PANEL_BORDERS);
    (inner_width > 0).then(|| Rect::new(x + 1, y, inner_width, 1))
}
