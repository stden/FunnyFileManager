//! Purpose: The F1 help screen: the key list drawn over the panels.

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::{centered_rect, dialog, far_block, palette};

pub(super) fn render_help(f: &mut Frame<'_>, top: usize) {
    let area = centered_rect(80, 100, f.area());
    dialog::render_shadow(f, area, f.area());

    let content = crate::models::help::HELP_LINES
        .iter()
        .copied()
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<Line<'_>>>();

    let style = Style::default().fg(palette::HELP_FG).bg(palette::HELP_BG);
    let block = far_block()
        .title(Line::from(" Help · ↑↓ PgUp/PgDn ").centered())
        .border_style(style)
        .style(style);

    let visible = usize::from(area.height.saturating_sub(2));
    let limit = crate::models::help::HELP_LINES
        .len()
        .saturating_sub(visible);
    let scroll = u16::try_from(top.min(limit)).unwrap_or(u16::MAX);
    let paragraph = Paragraph::new(content)
        .block(block)
        .style(style)
        .scroll((scroll, 0));

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
