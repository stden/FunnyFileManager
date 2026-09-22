//! Overlay selection colours on the syntax-highlighted, clipped editor body.

use ratatui::{layout::Rect, style::Style, Frame};

use crate::models::{CharColumn, Editor};

use super::{cells_before, palette, skip_cells, Cells};

pub(super) fn render(f: &mut Frame, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    let Some(selection) = editor.selection_range() else {
        return;
    };
    let style = Style::default()
        .fg(palette::CURSOR_FG)
        .bg(palette::CURSOR_BG);
    for (row, line) in editor
        .lines
        .iter()
        .enumerate()
        .skip(top)
        .take(usize::from(area.height))
    {
        if row < selection.start.line.get() || row > selection.end.line.get() {
            continue;
        }
        let (start, end) = selection_columns(&selection, row, line);
        if end <= skip {
            continue;
        }
        let dropped = skip_cells(line, skip).1;
        let first = cells_before(line, start).saturating_sub(dropped).get();
        let last = end
            .saturating_sub(dropped)
            .get()
            .min(usize::from(area.width));
        let y = area.y + (row - top) as u16;
        paint_selection(f, area.x, y, first..last, style);
    }
}

fn selection_columns(
    selection: &std::ops::Range<crate::search::Caret>,
    row: usize,
    line: &str,
) -> (CharColumn, Cells) {
    let start = if row == selection.start.line.get() {
        selection.start.column
    } else {
        CharColumn::new(0)
    };
    let end = if row == selection.end.line.get() {
        cells_before(line, selection.end.column)
    } else {
        // One extra cell makes a selected line break visible, including
        // on an empty line. It remains bounded by the visible body.
        Cells::of(line) + Cells::new(1)
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
        f.buffer_mut()[(left + x as u16, y)].set_style(style);
    }
}
