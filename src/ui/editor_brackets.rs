//! Purpose: Highlight the matching delimiter at the editor caret.

use super::{screen_u16, Cells, EditorLine};
use crate::models::Editor;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    Frame,
};

pub(super) fn render(frame: &mut Frame, editor: &Editor, area: Rect, top: usize, skip: Cells) {
    let Some((line, column, partner_line, partner_column)) = matching_pair(editor) else {
        return;
    };
    for (line_index, column_index) in [(line, column), (partner_line, partner_column)] {
        if line_index < top || line_index >= top.saturating_add(usize::from(area.height)) {
            continue;
        }
        let Some(text) = editor.lines.get(line_index) else {
            continue;
        };
        let display = EditorLine {
            text,
            tab: editor.tab_size,
        };
        let cell = display.cells_before(crate::models::CharColumn::new(column_index));
        let dropped = display.dropped(skip);
        let x = cell.saturating_sub(dropped);
        if x.get() < usize::from(area.width) {
            frame.buffer_mut()[(
                area.x.saturating_add(screen_u16(x.get())),
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

fn matching_pair(editor: &Editor) -> Option<(usize, usize, usize, usize)> {
    let candidate = candidate(editor)?;
    let positions: Vec<_> = editor
        .lines
        .iter()
        .enumerate()
        .flat_map(|(line, text)| {
            text.chars()
                .enumerate()
                .map(move |(column, ch)| (line, column, ch))
        })
        .collect();
    let index = positions
        .iter()
        .position(|(line, column, _)| (*line, *column) == (candidate.0, candidate.1))?;
    let partner = if is_open(candidate.2) {
        scan(positions[index + 1..].iter().copied(), candidate.2)?
    } else {
        scan(positions[..index].iter().copied().rev(), candidate.2)?
    };
    Some((candidate.0, candidate.1, partner.0, partner.1))
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
