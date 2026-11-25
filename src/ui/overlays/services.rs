//! Purpose: Shared rendering helpers and test frame capture for Jira, MCP, and AI overlays.

use std::ops::Range;

use ratatui::Frame;

#[cfg(test)]
use crate::models::App;
use crate::mouse::DialogMouse;
use crate::ui::dialog::{self, Ground, Row};

pub(in crate::ui) fn clip_cells(text: &str, max_w: usize) -> String {
    crate::ui::fit_name(text, crate::ui::Cells::new(max_w))
}

pub(in crate::ui) fn list_window(len: usize, cursor: usize, height: usize) -> Range<usize> {
    let start = cursor
        .saturating_sub(height / 2)
        .min(len.saturating_sub(height));
    start..start.saturating_add(height).min(len)
}

pub(in crate::ui) fn render_scrolled_lines(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    title: &str,
    lines: &[String],
    scroll: usize,
    empty_and_footer: (&str, &str),
    bounds: (usize, usize),
) -> Option<(u16, u16)> {
    let (empty, footer_text) = empty_and_footer;
    let (max_w, max_h) = bounds;
    let start = scroll.min(lines.len().saturating_sub(1));
    let end = start.saturating_add(max_h + 2).min(lines.len());
    let shown: Vec<String> = lines
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .map(|line| clip_cells(line, max_w))
        .collect();
    let footer = clip_cells(footer_text, max_w);
    let mut rows: Vec<Row<'_>> = shown.iter().map(|line| Row::Plain(line)).collect();
    if rows.is_empty() {
        rows.push(Row::Plain(empty));
    }
    rows.extend([Row::Divider, Row::Plain(&footer)]);
    dialog::render_tracked(f, tracker, title, Ground::Prompt, &rows, &[])
}

#[cfg(test)]
pub(in crate::ui) fn render_overlay_text(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, app))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content
        .chunks(120)
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
