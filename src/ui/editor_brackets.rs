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

fn matching_pair(editor: &Editor) -> Option<(usize, usize, usize, usize)> {
    let (line, column, bracket) = candidate(editor)?;
    let partner = if is_open(bracket) {
        scan(after(editor, line, column), bracket)?
    } else {
        scan(before(editor, line, column), bracket)?
    };
    Some((line, column, partner.0, partner.1))
}

/// Every character after (`line`, `column`), in reading order. Lazy, so the
/// scan stops at the partner instead of walking the whole buffer each frame.
fn after(
    editor: &Editor,
    line: usize,
    column: usize,
) -> impl Iterator<Item = (usize, usize, char)> + '_ {
    editor
        .lines
        .iter()
        .enumerate()
        .skip(line)
        .flat_map(move |(index, text)| {
            let first = if index == line {
                column.saturating_add(1)
            } else {
                0
            };
            text.chars()
                .enumerate()
                .skip(first)
                .map(move |(at, ch)| (index, at, ch))
        })
}

/// Every character before (`line`, `column`), nearest first.
fn before(
    editor: &Editor,
    line: usize,
    column: usize,
) -> impl Iterator<Item = (usize, usize, char)> + '_ {
    editor
        .lines
        .iter()
        .enumerate()
        .take(line.saturating_add(1))
        .rev()
        .flat_map(move |(index, text)| {
            let count = text.chars().count();
            let skip = if index == line {
                count.saturating_sub(column)
            } else {
                0
            };
            text.chars()
                .rev()
                .enumerate()
                .skip(skip)
                .map(move |(back, ch)| (index, count.saturating_sub(back.saturating_add(1)), ch))
        })
}

fn candidate(editor: &Editor) -> Option<(usize, usize, char)> {
    let cursor = (editor.cursor_line, editor.cursor_column().chars());
    editor
        .lines
        .get(cursor.0)?
        .chars()
        .nth(cursor.1)
        .filter(|ch| is_bracket(*ch))
        .map(|ch| (cursor.0, cursor.1, ch))
        .or_else(|| {
            cursor.1.checked_sub(1).and_then(|column| {
                editor
                    .lines
                    .get(cursor.0)?
                    .chars()
                    .nth(column)
                    .filter(|ch| is_bracket(*ch))
                    .map(|ch| (cursor.0, column, ch))
            })
        })
}

fn scan(
    positions: impl Iterator<Item = (usize, usize, char)>,
    bracket: char,
) -> Option<(usize, usize)> {
    let wanted = mate(bracket)?;
    let mut depth = 0usize;
    for (line, column, ch) in positions {
        if ch == bracket {
            depth += 1;
        }
        if ch == wanted {
            if depth == 0 {
                return Some((line, column));
            }
            depth -= 1;
        }
    }
    None
}

const fn is_bracket(ch: char) -> bool {
    mate(ch).is_some()
}
const fn is_open(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}
const fn mate(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        ')' => Some('('),
        '[' => Some(']'),
        ']' => Some('['),
        '{' => Some('}'),
        '}' => Some('{'),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
