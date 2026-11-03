//! Purpose: Highlight matching opening and closing XML/HTML tags in bright red
//! (`FarColorer` `def:PairStart` / `def:PairEnd` style) when the caret is on either tag.

mod scan;

#[cfg(test)]
mod tests_skip;

use super::super::{palette, screen_u16, Cells, EditorLine};
use super::byte_at_char;
use crate::models::{CharColumn, Editor};
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
    let Some(text) = editor.lines.get(span.line) else {
        return;
    };
    let display = EditorLine {
        text,
        tab: editor.tab_size,
    };
    let dropped = display.dropped(skip);
    let start = display.cells_before(CharColumn::new(span.start_col));
    let end = display.cells_before(CharColumn::new(span.end_col));
    if end <= dropped {
        return;
    }
    let first_x = start.saturating_sub(dropped).get();
    let last_x = end
        .saturating_sub(dropped)
        .get()
        .min(usize::from(area.width));
    let y = area.y.saturating_add(screen_u16(span.line - top));
    for x in first_x..last_x {
        frame.buffer_mut()[(area.x.saturating_add(screen_u16(x)), y)].set_fg(palette::CODE_URL);
    }
}

enum TagHint<'a> {
    SelfCloseOrClosing,
    Opening(&'a str),
}

pub(super) fn matching_xml_pair(editor: &Editor) -> Option<[TagSpan; 2]> {
    let cursor_line = editor.cursor_line;
    let cursor_col = editor.cursor_column().chars();
    let current = editor.lines.get(cursor_line)?;
    let hint = cursor_tag_hint(current, cursor_col)?;
    if let TagHint::Opening(name) = hint {
        let end = cursor_line
            .saturating_add(super::MAX_SCAN_LINES + 1)
            .min(editor.lines.len());
        let window = editor.lines.get(cursor_line..end).unwrap_or_default();
        if !window
            .iter()
            .any(|line| line.contains("/>") || contains_close_for(line, name))
        {
            return None;
        }
    }
    XmlScanner::default().find_pair(&editor.lines, cursor_line, cursor_col)
}

fn contains_close_for(line: &str, name: &str) -> bool {
    let mut rest = line;
    while let Some(pos) = rest.find("</") {
        let after = rest.get(pos + 2..).unwrap_or_default();
        if after.starts_with(name) {
            return true;
        }
        rest = after;
    }
    false
}

fn cursor_tag_hint(line: &str, cursor_col: usize) -> Option<TagHint<'_>> {
    if !line.contains('<') && !line.contains("/>") {
        return None;
    }
    let window_start = byte_at_char(line, cursor_col.saturating_sub(2));
    let window_end = byte_at_char(line, cursor_col.saturating_add(2));
    if line
        .get(window_start..window_end)
        .unwrap_or_default()
        .contains("/>")
    {
        return Some(TagHint::SelfCloseOrClosing);
    }
    let up_to_cursor = line
        .get(..byte_at_char(line, cursor_col.saturating_add(1)))
        .unwrap_or(line);
    let lt_pos = up_to_cursor.rfind('<')?;
    tag_hint_after_lt(line, lt_pos, cursor_col)
}

fn tag_hint_after_lt(line: &str, lt_pos: usize, cursor_col: usize) -> Option<TagHint<'_>> {
    let after_lt = line.get(lt_pos + 1..).unwrap_or_default();
    let before_prev = byte_at_char(line, cursor_col.saturating_sub(1));
    let between = line.get(lt_pos..before_prev).unwrap_or_default();
    if between.contains('>') {
        return None;
    }
    if after_lt.starts_with('/') {
        return Some(TagHint::SelfCloseOrClosing);
    }
    if between.contains(char::is_whitespace) {
        return None;
    }
    let name_len = after_lt
        .chars()
        .take_while(|&ch| scan::is_tag_name_char(ch))
        .map(char::len_utf8)
        .sum();
    (name_len > 0)
        .then(|| after_lt.get(..name_len))
        .flatten()
        .map(TagHint::Opening)
}
