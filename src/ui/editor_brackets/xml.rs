//! Purpose: Highlight matching opening and closing XML/HTML tags in bright red
//! (`FarColorer` `def:PairStart` / `def:PairEnd` style) when the caret is on either tag.

mod scan;

use super::super::{palette, screen_u16, Cells};
use super::column_on_screen;
use crate::models::Editor;
use ratatui::{layout::Rect, Frame};
use scan::XmlScanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TagSpan {
    pub(super) line: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
}

impl TagSpan {
    pub(super) const fn new(line: usize, start_col: usize, end_col: usize) -> Self {
        Self {
            line,
            start_col,
            end_col,
        }
    }

    pub(super) const fn contains(self, line: usize, col: usize) -> bool {
        self.line == line && col >= self.start_col && col < self.end_col
    }

    pub(super) fn matches_cursor(self, lines: &[String], line: usize, col: usize) -> bool {
        if self.contains(line, col) {
            return true;
        }
        if self.line == line && col == self.end_col {
            let next_ch = lines.get(line).and_then(|s| s.chars().nth(col));
            return next_ch.is_none_or(char::is_whitespace);
        }
        false
    }
}

pub(super) fn render(frame: &mut Frame<'_>, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    let Some(spans) = matching_xml_pair(editor) else {
        return;
    };
    for span in spans {
        paint_span(frame, editor, area, top, skip, span);
    }
}

fn paint_span(
    frame: &mut Frame<'_>,
    editor: &Editor,
    area: Rect,
    top: usize,
    skip: Cells,
    span: TagSpan,
) {
    if span.line < top || span.line >= top.saturating_add(usize::from(area.height)) {
        return;
    }
    let y = area.y.saturating_add(screen_u16(span.line - top));
    for col in span.start_col..span.end_col {
        let Some(x) = column_on_screen(editor, span.line, col, skip) else {
            continue;
        };
        if x < usize::from(area.width) {
            frame.buffer_mut()[(area.x.saturating_add(screen_u16(x)), y)].set_fg(palette::CODE_URL);
        }
    }
}

pub(super) fn matching_xml_pair(editor: &Editor) -> Option<[TagSpan; 2]> {
    let cursor_line = editor.cursor_line;
    let cursor_col = editor.cursor_column().chars();
    let current = editor.lines.get(cursor_line)?;
    if !current.contains('<') && !current.contains("/>") {
        return None;
    }
    XmlScanner::default().find_pair(&editor.lines, cursor_line, cursor_col)
}
