//! Purpose: Normalize, range-check, and group multi-edit LSP completions by line span.

use crate::lsp::{
    features::TextEdit,
    positions::{self, Position, Range},
};
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Point {
    pub(super) line: usize,
    pub(super) byte: usize,
}

pub(super) struct ResolvedEdit<'a> {
    pub(super) start: Point,
    pub(super) end: Point,
    pub(super) raw_range: Range,
    pub(super) replacement: Cow<'a, str>,
    pub(super) is_primary: bool,
}

pub(super) struct LineGroup<'a> {
    pub(super) start_line: usize,
    pub(super) start_u32: u32,
    pub(super) end_line: usize,
    pub(super) edits: Vec<ResolvedEdit<'a>>,
}

pub(super) fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

fn clash(earlier: &std::ops::Range<Point>, later: &std::ops::Range<Point>) -> bool {
    let same_start = earlier.start == later.start;
    same_start || later.start < earlier.end
}

fn resolve_point(
    lines: &[String],
    trailing_newline: bool,
    pos: Position,
) -> Result<Point, &'static str> {
    let line = usize::try_from(pos.line).map_err(|_err| "Invalid completion range")?;
    if line == lines.len() && trailing_newline && pos.character == 0 {
        return Ok(Point { line, byte: 0 });
    }
    let raw = lines.get(line).ok_or("Invalid completion range")?;
    let visible = if line + 1 < lines.len() || trailing_newline {
        raw.strip_suffix('\r').unwrap_or(raw)
    } else {
        raw.as_str()
    };
    let byte = positions::column_byte(visible, pos.character).ok_or("Invalid completion range")?;
    Ok(Point { line, byte })
}

fn resolve_edit<'a>(
    lines: &[String],
    trailing_newline: bool,
    edit: &'a TextEdit,
    is_primary: bool,
) -> Result<ResolvedEdit<'a>, &'static str> {
    let start = resolve_point(lines, trailing_newline, edit.range.start)?;
    let end = resolve_point(lines, trailing_newline, edit.range.end)?;
    if start > end {
        return Err("Invalid completion range");
    }
    Ok(ResolvedEdit {
        start,
        end,
        raw_range: edit.range,
        replacement: normalize(&edit.new_text),
        is_primary,
    })
}

pub(super) fn prepare_groups<'a>(
    lines: &[String],
    trailing_newline: bool,
    primary: &'a TextEdit,
    extra: &'a [TextEdit],
) -> Result<Vec<LineGroup<'a>>, &'static str> {
    let mut resolved = Vec::with_capacity(1 + extra.len());
    resolved.push(resolve_edit(lines, trailing_newline, primary, true)?);
    for edit in extra {
        resolved.push(resolve_edit(lines, trailing_newline, edit, false)?);
    }
    resolved.sort_unstable_by_key(|e| (e.start, e.end));
    if resolved
        .iter()
        .zip(resolved.iter().skip(1))
        .any(|(a, b)| clash(&(a.start..a.end), &(b.start..b.end)))
    {
        return Err("Overlapping completion edits");
    }
    Ok(group_by_lines(resolved))
}

fn group_by_lines(edits: Vec<ResolvedEdit<'_>>) -> Vec<LineGroup<'_>> {
    let mut groups: Vec<LineGroup<'_>> = Vec::new();
    for edit in edits {
        let end_line = edit.end.line + 1;
        if let Some(last) = groups.last_mut().filter(|g| edit.start.line < g.end_line) {
            last.end_line = last.end_line.max(end_line);
            last.edits.push(edit);
        } else {
            groups.push(LineGroup {
                start_line: edit.start.line,
                start_u32: edit.raw_range.start.line,
                end_line,
                edits: vec![edit],
            });
        }
    }
    groups
}
