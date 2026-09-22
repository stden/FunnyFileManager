//! Content-sized vertical menus with shared rendering and click geometry.

use crate::models::MenuChoice;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::{dialog, far_block, palette, Cells};

pub fn row_at(
    screen: Rect,
    title: &str,
    rows: &[MenuChoice],
    selected: usize,
    column: u16,
    row: u16,
) -> Option<usize> {
    let body = far_block().inner(area(screen, title, rows));
    if !body.contains((column, row).into()) {
        return None;
    }
    let index = first_row(selected, rows.len(), body.height) + usize::from(row - body.y);
    (index < rows.len()).then_some(index)
}

fn area(screen: Rect, title: &str, rows: &[MenuChoice]) -> Rect {
    let content = rows
        .iter()
        .map(|row| Cells::of(&row.label).get())
        .chain([Cells::of(title).get(), 22])
        .max()
        .unwrap_or(22);
    let width = u16::try_from(content.saturating_add(4))
        .unwrap_or(u16::MAX)
        .min(80)
        .min(screen.width.saturating_sub(4).max(2).min(screen.width));
    let height = u16::try_from(rows.len().max(1).saturating_add(2))
        .unwrap_or(u16::MAX)
        .min(screen.height.saturating_sub(4).max(3).min(screen.height));
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

fn first_row(selected: usize, length: usize, height: u16) -> usize {
    selected
        .min(length.saturating_sub(1))
        .saturating_add(1)
        .saturating_sub(usize::from(height))
}

pub(super) fn render(f: &mut Frame, title: &str, rows: &[MenuChoice], selected: usize) {
    let area = area(f.area(), title, rows);
    dialog::render_shadow(f, area, f.area());
    let style = Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG);
    let block = far_block()
        .title(Line::from(format!(" {title} ")).centered())
        .style(style);
    let body = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    if rows.is_empty() {
        f.render_widget(Paragraph::new(" No entries. F10: close").style(style), body);
        return;
    }
    render_rows(f, body, rows, selected, style);
}

fn render_rows(f: &mut Frame, body: Rect, rows: &[MenuChoice], selected: usize, style: Style) {
    let first = first_row(selected, rows.len(), body.height);
    for (index, row) in rows
        .iter()
        .enumerate()
        .skip(first)
        .take(usize::from(body.height))
    {
        let area = Rect::new(
            body.x,
            body.y + u16::try_from(index - first).unwrap_or(0),
            body.width,
            1,
        );
        let style = if index == selected {
            style.bg(palette::MENU_SELECTED_BG)
        } else {
            style
        };
        f.render_widget(Paragraph::new(format!(" {}", row.label)).style(style), area);
    }
}
