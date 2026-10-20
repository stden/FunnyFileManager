//! Purpose: Highlight the matching delimiter at the editor caret.

use super::{screen_u16, Cells, EditorLine};
use crate::models::Editor;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

mod xml;

pub(super) fn render(frame: &mut Frame<'_>, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    xml::render(frame, editor, area, top, skip);
    let Some((line, column, partner_line, partner_column)) = matching_pair(editor) else {
        return;
    };
    for (line_index, column_index) in [(line, column), (partner_line, partner_column)] {
        if line_index < top || line_index >= top.saturating_add(usize::from(area.height)) {
            continue;
        }
        let Some(x) = column_on_screen(editor, line_index, column_index, skip) else {
            continue;
        };
        if x < usize::from(area.width) {
            frame.buffer_mut()[(
                area.x.saturating_add(screen_u16(x)),
                area.y.saturating_add(screen_u16(line_index - top)),
            )]
                .set_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                );
        }
    }
}

/// The screen column of the character at `column_index` on `line_index` once
/// `skip` cells are scrolled off; `None` for a missing line or for a character
/// scrolled off the left edge, which clamping to column 0 would paint over
/// whatever shows there.
pub(super) fn column_on_screen(
    editor: &Editor,
    line_index: usize,
    column_index: usize,
    skip: Cells,
) -> Option<usize> {
    let display = EditorLine {
        text: editor.lines.get(line_index)?,
        tab: editor.tab_size,
    };
    let cell = display.cells_before(crate::models::CharColumn::new(column_index));
    cell.get().checked_sub(display.dropped(skip).get())
}

pub(super) const MAX_SCAN_LINES: usize = 1_000;

fn matching_pair(editor: &Editor) -> Option<(usize, usize, usize, usize)> {
    let (line, column, bracket) = candidate(editor)?;
    let (bracket_byte, wanted_byte) = mate(bracket)?;
    let partner = if is_open(bracket) {
        scan_forward(editor, line, column, bracket_byte, wanted_byte)?
    } else {
        scan_backward(editor, line, column, bracket_byte, wanted_byte)?
    };
    Some((line, column, partner.0, partner.1))
}

fn scan_forward(
    editor: &Editor,
    line: usize,
    column: usize,
    bracket: u8,
    wanted: u8,
) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for (index, text) in editor
        .lines
        .iter()
        .enumerate()
        .skip(line)
        .take(MAX_SCAN_LINES)
    {
        let start = if index == line {
            byte_at_char(text, column.saturating_add(1))
        } else {
            0
        };
        let slice = text.get(start..).unwrap_or_default().as_bytes();
        for (offset, &byte) in slice.iter().enumerate() {
            if step_bracket(byte, bracket, wanted, &mut depth) {
                let col = text.get(..start + offset)?.chars().count();
                return Some((index, col));
            }
        }
    }
    None
}

fn scan_backward(
    editor: &Editor,
    line: usize,
    column: usize,
    bracket: u8,
    wanted: u8,
) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let first_line = line.saturating_sub(MAX_SCAN_LINES);
    for (index, text) in editor
        .lines
        .iter()
        .enumerate()
        .take(line.saturating_add(1))
        .skip(first_line)
        .rev()
    {
        let end = if index == line {
            byte_at_char(text, column)
        } else {
            text.len()
        };
        let slice = text.get(..end).unwrap_or_default().as_bytes();
        for (offset, &byte) in slice.iter().enumerate().rev() {
            if step_bracket(byte, bracket, wanted, &mut depth) {
                let col = text.get(..offset)?.chars().count();
                return Some((index, col));
            }
        }
    }
    None
}

const fn step_bracket(byte: u8, bracket: u8, wanted: u8, depth: &mut usize) -> bool {
    if byte == bracket {
        *depth += 1;
    } else if byte == wanted {
        if *depth == 0 {
            return true;
        }
        *depth -= 1;
    }
    false
}

pub(super) fn byte_at_char(text: &str, char_col: usize) -> usize {
    if text.is_ascii() {
        return char_col.min(text.len());
    }
    text.char_indices()
        .nth(char_col)
        .map_or(text.len(), |(idx, _)| idx)
}

fn candidate(editor: &Editor) -> Option<(usize, usize, char)> {
    let line_idx = editor.cursor_line;
    let col = editor.cursor_column().chars();
    let line = editor.lines.get(line_idx)?;
    if col == 0 {
        let ch = line.chars().next().filter(|ch| is_bracket(*ch))?;
        return Some((line_idx, 0, ch));
    }
    let mut chars = line.chars().skip(col - 1);
    let prev = chars.next();
    let curr = chars.next();
    curr.filter(|ch| is_bracket(*ch))
        .map(|ch| (line_idx, col, ch))
        .or_else(|| {
            prev.filter(|ch| is_bracket(*ch))
                .map(|ch| (line_idx, col - 1, ch))
        })
}

const fn is_bracket(ch: char) -> bool {
    mate(ch).is_some()
}
const fn is_open(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}
const fn mate(ch: char) -> Option<(u8, u8)> {
    match ch {
        '(' => Some((b'(', b')')),
        ')' => Some((b')', b'(')),
        '[' => Some((b'[', b']')),
        ']' => Some((b']', b'[')),
        '{' => Some((b'{', b'}')),
        '}' => Some((b'}', b'{')),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
