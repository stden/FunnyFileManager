//! Purpose: Scan up to `MAX_SCAN_LINES` around the caret for matching `()`, `[]`, or `{}` delimiters.

use crate::models::{pairs::BracketPos, Editor};

use super::MAX_SCAN_LINES;

pub(super) fn matching_pair(editor: &Editor) -> Option<[BracketPos; 2]> {
    let (pos, bracket) = candidate(editor)?;
    let (bracket_byte, wanted_byte) = mate(bracket)?;
    let partner = if matches!(bracket, '(' | '[' | '{') {
        scan_forward(editor, pos, bracket_byte, wanted_byte)?
    } else {
        scan_backward(editor, pos, bracket_byte, wanted_byte)?
    };
    Some([pos, partner])
}

fn scan_forward(
    editor: &Editor,
    origin: BracketPos,
    bracket: u8,
    wanted: u8,
) -> Option<BracketPos> {
    let mut depth = 0_usize;
    for (index, text) in editor
        .lines
        .iter()
        .enumerate()
        .skip(origin.line)
        .take(MAX_SCAN_LINES)
    {
        let start = if index == origin.line {
            byte_at_char(text, origin.column.saturating_add(1))
        } else {
            0
        };
        let slice = text.get(start..).unwrap_or_default().as_bytes();
        for offset in memchr::memchr2_iter(bracket, wanted, slice) {
            let byte = slice.get(offset).copied().unwrap_or_default();
            if closes_outer_pair(byte, bracket, &mut depth) {
                return Some(BracketPos {
                    line: index,
                    column: char_col_at(text, start + offset)?,
                });
            }
        }
    }
    None
}

fn scan_backward(
    editor: &Editor,
    origin: BracketPos,
    bracket: u8,
    wanted: u8,
) -> Option<BracketPos> {
    let mut depth = 0_usize;
    let first_line = origin.line.saturating_sub(MAX_SCAN_LINES);
    for (index, text) in editor
        .lines
        .iter()
        .enumerate()
        .take(origin.line.saturating_add(1))
        .skip(first_line)
        .rev()
    {
        let end = if index == origin.line {
            byte_at_char(text, origin.column)
        } else {
            text.len()
        };
        let slice = text.get(..end).unwrap_or_default().as_bytes();
        for offset in memchr::memrchr2_iter(bracket, wanted, slice) {
            let byte = slice.get(offset).copied().unwrap_or_default();
            if closes_outer_pair(byte, bracket, &mut depth) {
                return Some(BracketPos {
                    line: index,
                    column: char_col_at(text, offset)?,
                });
            }
        }
    }
    None
}

const fn closes_outer_pair(byte: u8, same_bracket: u8, depth: &mut usize) -> bool {
    if byte == same_bracket {
        *depth += 1;
        return false;
    }
    if *depth == 0 {
        return true;
    }
    *depth -= 1;
    false
}

fn char_col_at(text: &str, byte_offset: usize) -> Option<usize> {
    let head = text.get(..byte_offset)?;
    if head.is_ascii() {
        return Some(byte_offset);
    }
    Some(head.chars().count())
}

pub(super) fn byte_at_char(text: &str, char_col: usize) -> usize {
    if text.is_ascii() {
        return char_col.min(text.len());
    }
    text.char_indices()
        .nth(char_col)
        .map_or(text.len(), |(idx, _)| idx)
}

fn chars_before_and_at(line: &str, col: usize) -> (Option<char>, Option<char>) {
    if col == 0 {
        return (None, line.chars().next());
    }
    let mut chars = line.chars().skip(col - 1);
    (chars.next(), chars.next())
}

fn candidate(editor: &Editor) -> Option<(BracketPos, char)> {
    let line_idx = editor.cursor_line;
    let col = editor.cursor_column().chars();
    let line = editor.lines.get(line_idx)?;
    let (before, at_cursor) = chars_before_and_at(line, col);
    if let Some(ch) = at_cursor.filter(|&ch| mate(ch).is_some()) {
        return Some((
            BracketPos {
                line: line_idx,
                column: col,
            },
            ch,
        ));
    }
    if let Some(ch) = before.filter(|&ch| mate(ch).is_some()) {
        return Some((
            BracketPos {
                line: line_idx,
                column: col - 1,
            },
            ch,
        ));
    }
    None
}

pub(super) const fn mate(ch: char) -> Option<(u8, u8)> {
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
