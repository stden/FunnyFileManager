//! Purpose: Strict conversion between UTF-8 bytes, editor characters, and LSP UTF-16.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

pub fn from_chars(line: usize, text: &str, column: usize) -> Option<Position> {
    if column > text.chars().count() {
        return None;
    }
    Some(Position {
        line: u32::try_from(line).ok()?,
        character: u32::try_from(
            text.chars()
                .take(column)
                .map(char::len_utf16)
                .sum::<usize>(),
        )
        .ok()?,
    })
}

pub fn to_chars(text: &str, units: u32) -> Option<usize> {
    let byte = column_byte(text, units)?;
    Some(text.get(..byte)?.chars().count())
}

fn column_byte(text: &str, units: u32) -> Option<usize> {
    let mut remaining = usize::try_from(units).ok()?;
    for (byte, character) in text.char_indices() {
        if remaining == 0 {
            return Some(byte);
        }
        remaining = remaining.checked_sub(character.len_utf16())?;
    }
    (remaining == 0).then_some(text.len())
}

pub fn byte_offset(text: &str, position: Position) -> Option<usize> {
    let mut base = 0;
    for (line, content) in text.split('\n').enumerate() {
        if line == usize::try_from(position.line).ok()? {
            return column_byte(
                content.strip_suffix('\r').unwrap_or(content),
                position.character,
            )
            .map(|column| base + column);
        }
        base += content.len() + 1;
    }
    None
}

pub fn at_byte(text: &str, offset: usize) -> Option<Position> {
    let (prefix, rest) = text.split_at_checked(offset)?;
    if prefix.ends_with('\r') && rest.starts_with('\n') {
        return None;
    }
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let column = prefix.rsplit('\n').next()?;
    from_chars(line, column, column.chars().count())
}

pub fn byte_range(text: &str, range: Range) -> Option<std::ops::Range<usize>> {
    let start = byte_offset(text, range.start)?;
    let end = byte_offset(text, range.end)?;
    (start <= end).then_some(start..end)
}
