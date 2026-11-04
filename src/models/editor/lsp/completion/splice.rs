//! Purpose: Splice grouped multi-line completion edits into buffer lines and highlight cache.

use super::{
    prepare::{LineGroup, ResolvedEdit},
    Editor,
};
use crate::lsp::positions::{self, Position, Range};

#[derive(Clone, Copy, Default)]
pub(super) struct CaretPos {
    pub(super) line: usize,
    pub(super) column: usize,
}

/// Tracks line shifts and the primary caret while splicing groups from bottom to top.
#[derive(Default)]
struct ReverseSpliceTracker {
    caret: Option<CaretPos>,
    total_added: usize,
    total_removed: usize,
}

impl ReverseSpliceTracker {
    fn record(&mut self, group: &LineGroup<'_>, added: usize, group_caret: Option<CaretPos>) {
        let removed = group.end_line - group.start_line;
        self.total_added += added;
        self.total_removed += removed;
        if let Some(local) = group_caret {
            self.caret = Some(CaretPos {
                line: group.start_line + local.line,
                column: local.column,
            });
        } else if let Some(caret) = &mut self.caret {
            caret.line = caret.line + added - removed;
        }
    }

    fn dirty_span(&self, groups: &[LineGroup<'_>], line_count: usize) -> (usize, usize) {
        let first = groups.first().map_or(0, |g| g.start_line);
        let last_end = groups
            .last()
            .map_or(1, |g| g.end_line + self.total_added - self.total_removed)
            .min(line_count.max(1));
        (first, last_end)
    }
}

struct ChunkEdit<'a> {
    byte_range: std::ops::Range<usize>,
    replacement: &'a str,
    is_primary: bool,
}

impl Editor {
    pub(super) fn splice_completion_groups(&mut self, groups: &[LineGroup<'_>]) -> CaretPos {
        let orig_len = self.lines.len();
        let pushed_eof = groups.last().is_some_and(|g| g.end_line > orig_len);
        if pushed_eof {
            self.lines.push(String::new());
        }
        let mut tracker = ReverseSpliceTracker::default();
        for group in groups.iter().rev() {
            self.apply_completion_group(group, orig_len, &mut tracker);
        }
        let caret = tracker.caret.unwrap_or_default();
        self.trim_virtual_eof(pushed_eof, caret.line);
        let (first, last_end) = tracker.dirty_span(groups, self.lines.len());
        self.note_dirty(first, last_end);
        caret
    }

    fn apply_completion_group(
        &mut self,
        group: &LineGroup<'_>,
        orig_len: usize,
        tracker: &mut ReverseSpliceTracker,
    ) {
        let slice = self
            .lines
            .get(group.start_line..group.end_line)
            .unwrap_or_default();
        let (replacement, group_caret) = render_group(slice, group);
        tracker.record(group, replacement.len(), group_caret);
        self.highlights.borrow_mut().splice_lines(
            group.start_line,
            group.end_line.min(orig_len),
            &replacement,
        );
        self.lines
            .splice(group.start_line..group.end_line, replacement);
    }

    fn trim_virtual_eof(&mut self, pushed_eof: bool, caret_line: usize) {
        if pushed_eof
            && self.lines.len() > 1
            && caret_line + 1 < self.lines.len()
            && self.lines.last().is_some_and(String::is_empty)
        {
            self.lines.pop();
            let len = self.lines.len();
            self.highlights.borrow_mut().splice_lines(len, len + 1, &[]);
        }
    }
}

fn render_group(slice: &[String], group: &LineGroup<'_>) -> (Vec<String>, Option<CaretPos>) {
    let starts: Vec<usize> = slice
        .iter()
        .scan(0_usize, |acc, line| {
            let cur = *acc;
            *acc += line.len() + 1;
            Some(cur)
        })
        .collect();
    let mut chunk = slice.join("\n");
    let edits: Vec<ChunkEdit<'_>> = group
        .edits
        .iter()
        .map(|edit| ChunkEdit {
            byte_range: local_range(&chunk, &starts, group, edit),
            replacement: edit.replacement.as_ref(),
            is_primary: edit.is_primary,
        })
        .collect();
    let caret_byte = compute_group_caret(&edits);
    for edit in edits.iter().rev() {
        chunk.replace_range(edit.byte_range.clone(), edit.replacement);
    }
    let caret = caret_byte.map(|at| caret_in_chunk(&chunk, at));
    (chunk.split('\n').map(str::to_owned).collect(), caret)
}

fn local_range(
    chunk: &str,
    starts: &[usize],
    group: &LineGroup<'_>,
    edit: &ResolvedEdit<'_>,
) -> std::ops::Range<usize> {
    let rel = Range {
        start: Position {
            line: edit.raw_range.start.line - group.start_u32,
            character: edit.raw_range.start.character,
        },
        end: Position {
            line: edit.raw_range.end.line - group.start_u32,
            character: edit.raw_range.end.character,
        },
    };
    let start = starts
        .get(edit.start.line - group.start_line)
        .copied()
        .unwrap_or(0)
        + edit.start.byte;
    let end = starts
        .get(edit.end.line - group.start_line)
        .copied()
        .unwrap_or(0)
        + edit.end.byte;
    positions::byte_range(chunk, rel).unwrap_or(start..end)
}

fn compute_group_caret(edits: &[ChunkEdit<'_>]) -> Option<usize> {
    let primary = edits.iter().find(|edit| edit.is_primary)?;
    let mut caret = primary.byte_range.start + primary.replacement.len();
    for edit in edits {
        if edit.byte_range.start < primary.byte_range.start {
            caret = caret - edit.byte_range.len() + edit.replacement.len();
        }
    }
    Some(caret)
}

fn caret_in_chunk(chunk: &str, byte_offset: usize) -> CaretPos {
    let before = chunk.get(..byte_offset).unwrap_or(chunk);
    let line = memchr::memchr_iter(b'\n', before.as_bytes()).count();
    let line_start = memchr::memrchr(b'\n', before.as_bytes()).map_or(0, |idx| idx + 1);
    let column = before.get(line_start..).unwrap_or_default().chars().count();
    CaretPos { line, column }
}
