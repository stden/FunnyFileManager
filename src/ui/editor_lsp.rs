//! Purpose: Render language-server popups, diagnostic ranges, and compact editor feedback.

use super::{cells_before, skip_cells, Cells};
use crate::lsp::{features::Diagnostic, positions};
use crate::models::{lsp::Dialog, CharColumn, Editor, ServerState};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::fmt::Write;

pub(super) fn status(editor: &Editor) -> String {
    if editor.lsp.state == ServerState::Stopped {
        return String::new();
    }
    let cursor = editor.lsp_position();
    let diagnostic = editor.lsp.diagnostics.iter().find(|diagnostic| {
        cursor.is_some_and(|at| diagnostic.range.start <= at && at <= diagnostic.range.end)
    });
    diagnostic.map_or_else(
        || {
            format!(
                " | {} [{}]",
                editor.lsp.state.label(),
                editor.lsp.diagnostics.len()
            )
        },
        |diagnostic| format!(" | {}: {}", diagnostic.label(), diagnostic.message),
    )
}

pub(super) fn label(editor: &Editor, marker: &str) -> String {
    let path = crate::path_display::label(&editor.path);
    let status = status(editor);
    if status.is_empty() {
        format!(" {path}{marker}")
    } else {
        format!(" {} | {path}{marker}", status.trim_start_matches(" | "))
    }
}

pub(super) fn popup(frame: &mut Frame, editor: &Editor) -> bool {
    let Some(dialog) = &editor.lsp.popup else {
        return false;
    };
    let screen = frame.area();
    let width = screen.width.min(86);
    let height = screen.height.min(18);
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    let (title, text, scroll) = contents(dialog);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Cyan).bg(Color::Rgb(0, 0, 128)))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
    true
}

fn contents(dialog: &Dialog) -> (&'static str, String, u16) {
    match dialog {
        Dialog::Hover { text, scroll } => (" Hover — ↑↓ scroll, Esc close ", text.clone(), *scroll),
        Dialog::Completion {
            items, selected, ..
        } => {
            let text = completions(items, *selected);
            (" Completion — ↑↓ select, Enter apply, Esc close ", text, 0)
        }
        Dialog::Definitions { items, selected } => {
            let text = definitions(items, *selected);
            (" Definitions — ↑↓ select, Enter open, Esc close ", text, 0)
        }
    }
}

fn completions(items: &[crate::lsp::features::Completion], selected: usize) -> String {
    let mut text = String::new();
    for (index, item) in items
        .iter()
        .enumerate()
        .skip(selected.saturating_sub(5))
        .take(8)
    {
        writeln!(
            text,
            "{} {} [{}]",
            marker(index, selected),
            item.label,
            item.kind.map_or(String::new(), |kind| kind.to_string())
        )
        .expect("writing to a String cannot fail");
    }
    if let Some(item) = items.get(selected) {
        write!(text, "\n{}\n{}", item.detail, item.documentation)
            .expect("writing to a String cannot fail");
    }
    text
}

fn definitions(items: &[crate::lsp::features::Definition], selected: usize) -> String {
    let mut text = String::new();
    for (index, item) in items
        .iter()
        .enumerate()
        .skip(selected.saturating_sub(5))
        .take(12)
    {
        writeln!(
            text,
            "{} {}:{}:{}",
            marker(index, selected),
            crate::path_display::label(&item.path),
            item.range.start.line + 1,
            item.range.start.character + 1
        )
        .expect("writing to a String cannot fail");
    }
    text
}

const fn marker(index: usize, selected: usize) -> &'static str {
    if index == selected {
        ">"
    } else {
        " "
    }
}

pub(super) fn diagnostics(frame: &mut Frame, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    for diagnostic in &editor.lsp.diagnostics {
        for (row, line) in editor
            .lines
            .iter()
            .enumerate()
            .skip(top)
            .take(usize::from(area.height))
        {
            if let Some(columns) = diagnostic_columns(diagnostic, row, line, skip, area.width) {
                paint(
                    frame,
                    area.x,
                    area.y + (row - top) as u16,
                    columns,
                    diagnostic.severity,
                );
            }
        }
    }
}

fn diagnostic_columns(
    diagnostic: &Diagnostic,
    row: usize,
    line: &str,
    skip: Cells,
    width: u16,
) -> Option<std::ops::Range<usize>> {
    let range = diagnostic.range;
    if range.start > range.end || row < range.start.line as usize || row > range.end.line as usize {
        return None;
    }
    let first = if row == range.start.line as usize {
        positions::to_chars(line, range.start.character)?
    } else {
        0
    };
    let last = if row == range.end.line as usize {
        positions::to_chars(line, range.end.character)?
    } else {
        line.chars().count()
    };
    let dropped = skip_cells(line, skip).1;
    let start = cells_before(line, CharColumn::new(first));
    let end = cells_before(line, CharColumn::new(last)).max(start + Cells::new(1));
    if end <= dropped {
        return None;
    }
    Some(
        start.saturating_sub(dropped).get()
            ..end.saturating_sub(dropped).get().min(usize::from(width)),
    )
}

fn paint(
    frame: &mut Frame,
    x: u16,
    y: u16,
    columns: std::ops::Range<usize>,
    severity: Option<u32>,
) {
    let color = match severity {
        Some(1) => Color::LightRed,
        Some(2) => Color::Yellow,
        _ => Color::Cyan,
    };
    let style = Style::default()
        .fg(color)
        .add_modifier(Modifier::UNDERLINED);
    for column in columns {
        frame.buffer_mut()[(x + column as u16, y)].set_style(style);
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_capture;
