//! Overlay selection colours on the syntax-highlighted, clipped editor body.

use ratatui::{layout::Rect, style::Style, Frame};

use crate::models::{CharColumn, Editor};

use super::{palette, screen_u16, Cells, EditorLine};

pub(super) fn render(f: &mut Frame, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    let Some(selection) = editor.selection_range() else {
        return;
    };
    let style = Style::default()
        .fg(palette::CURSOR_FG)
        .bg(palette::CURSOR_BG);
    let selected_rows = selection.start.line.get()..=selection.end.line.get();
    for (row, text) in editor
        .lines
        .iter()
        .enumerate()
        .skip(top)
        .take(usize::from(area.height))
        .filter(|(row, _)| selected_rows.contains(row))
    {
        let line = EditorLine {
            text,
            tab: editor.tab_size,
        };
        if let Some(columns) = visible_columns(&selection, row, line, skip, area.width) {
            let y = area.y.saturating_add(screen_u16(row - top));
            paint_selection(f, area.x, y, columns, style);
        }
    }
}

/// The screen columns the selected part of `line` covers once the window has
/// skipped `skip` cells; `None` when all of it is scrolled off to the left.
fn visible_columns(
    selection: &std::ops::Range<crate::search::Caret>,
    row: usize,
    line: EditorLine<'_>,
    skip: Cells,
    width: u16,
) -> Option<std::ops::Range<usize>> {
    let (start, end) = selection_columns(selection, row, line);
    if end <= skip {
        return None;
    }
    let dropped = line.dropped(skip);
    let first = line.cells_before(start).saturating_sub(dropped).get();
    let last = end.saturating_sub(dropped).get().min(usize::from(width));
    Some(first..last)
}

fn selection_columns(
    selection: &std::ops::Range<crate::search::Caret>,
    row: usize,
    line: EditorLine<'_>,
) -> (CharColumn, Cells) {
    let start = if row == selection.start.line.get() {
        selection.start.column
    } else {
        CharColumn::new(0)
    };
    let end = if row == selection.end.line.get() {
        line.cells_before(selection.end.column)
    } else {
        // One extra cell makes a selected line break visible, including
        // on an empty line. It remains bounded by the visible body.
        line.width() + Cells::new(1)
    };
    (start, end)
}

#[cfg(test)]
mod tests;

fn paint_selection(
    f: &mut Frame,
    left: u16,
    y: u16,
    columns: std::ops::Range<usize>,
    style: Style,
) {
    for x in columns {
        f.buffer_mut()[(left.saturating_add(screen_u16(x)), y)].set_style(style);
    }
}
