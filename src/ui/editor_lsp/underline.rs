//! Purpose: Underline the editor text a language server's diagnostics cover,
//! in the colour of their severity, clipped to the visible columns.

use super::super::{screen_u16, Cells, EditorLine};
use crate::lsp::{features::Diagnostic, positions};
use crate::models::{CharColumn, Editor};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

pub(in crate::ui) fn diagnostics(
    frame: &mut Frame<'_>,
    editor: &Editor,
    area: Rect,
    top: usize,
    skip: Cells,
) {
    for diagnostic in &editor.lsp.diagnostics {
        for (row, line) in editor
            .lines
            .iter()
            .enumerate()
            .skip(top)
            .take(usize::from(area.height))
        {
            let line = EditorLine {
                text: line,
                tab: editor.tab_size,
            };
            if let Some(columns) = diagnostic_columns(diagnostic, row, line, skip, area.width) {
                paint(
                    frame,
                    area.x,
                    area.y.saturating_add(screen_u16(row - top)),
                    columns,
                    diagnostic.severity,
                );
            }
        }
    }
}

pub(super) fn diagnostic_columns(
    diagnostic: &Diagnostic,
    row: usize,
    line: EditorLine<'_>,
    skip: Cells,
    width: u16,
) -> Option<std::ops::Range<usize>> {
    let range = diagnostic.range;
    let start_line = usize::try_from(range.start.line).ok()?;
    let end_line = usize::try_from(range.end.line).ok()?;
    if range.start > range.end || row < start_line || row > end_line {
        return None;
    }
    let first = if row == start_line {
        positions::to_chars(line.text, range.start.character)?
    } else {
        0
    };
    let last = if row == end_line {
        positions::to_chars(line.text, range.end.character)?
    } else {
        line.text.chars().count()
    };
    let dropped = line.dropped(skip);
    let start = line.cells_before(CharColumn::new(first));
    let end = line
        .cells_before(CharColumn::new(last))
        .max(start + Cells::new(1));
    if end <= dropped {
        return None;
    }
    Some(
        start.saturating_sub(dropped).get()
            ..end.saturating_sub(dropped).get().min(usize::from(width)),
    )
}

pub(super) fn paint(
    frame: &mut Frame<'_>,
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
        frame.buffer_mut()[(x.saturating_add(screen_u16(column)), y)].set_style(style);
    }
}
