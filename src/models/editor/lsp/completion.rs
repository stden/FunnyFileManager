//! Purpose: Validate and atomically apply Unicode completion edits to the editor buffer.

use super::Editor;
use crate::lsp::{
    features::{Completion, TextEdit},
    positions::{self, Position, Range},
    types::Query,
};
use std::borrow::Cow;

impl Editor {
    pub(crate) fn complete_lsp(
        &mut self,
        query: Query,
        item: &Completion,
    ) -> Result<(), &'static str> {
        self.apply_completion(query, item, false)
    }

    pub(crate) fn complete_lsp_replace(
        &mut self,
        query: Query,
        item: &Completion,
    ) -> Result<(), &'static str> {
        self.apply_completion(query, item, true)
    }

    fn apply_completion(
        &mut self,
        query: Query,
        item: &Completion,
        overwrite_suffix: bool,
    ) -> Result<(), &'static str> {
        if self.lsp_stamp() != query.stamp || self.lsp_position() != Some(query.position) {
            return Err("Completion expired; request it again");
        }
        if item.snippet {
            return Err("Snippet completion is not supported");
        }
        let mut text = self.lsp_text();
        let primary = self.primary_edit(item, query, overwrite_suffix);
        let (edits, caret) = prepare(&text, &primary, &item.additional_edits)?;
        for (range, replacement) in edits.into_iter().rev() {
            text.replace_range(range, &replacement);
        }
        let position = positions::at_byte(&text, caret).ok_or("Invalid completion cursor")?;
        self.set_completed_text(&text, caret);
        let (line, column) = self.caret_at(position).ok_or("Invalid completion cursor")?;
        self.cursor_line = line;
        self.cursor_col = column;
        self.selection = super::super::EditorSelection::None;
        self.lsp.completions = None;
        self.mark_modified();
        Ok(())
    }

    fn primary_edit(&self, item: &Completion, query: Query, overwrite_suffix: bool) -> TextEdit {
        let mut primary = item
            .edit
            .clone()
            .unwrap_or_else(|| self.fallback_edit(item, query));
        if overwrite_suffix {
            if let Some(end) = self.identifier_end_position() {
                primary.range.end = primary.range.end.max(end);
            }
        }
        primary
    }

    fn set_completed_text(&mut self, text: &str, caret: usize) {
        // Keep the final empty line only when the file has no trailing newline
        // or the inserted text places the caret there.
        let content = if self.format.trailing_newline && caret < text.len() {
            text.strip_suffix('\n').unwrap_or(text)
        } else {
            text
        };
        self.lines = content.split('\n').map(str::to_owned).collect();
    }

    /// The line index and character column `position` names in the buffer.
    fn caret_at(&self, position: Position) -> Option<(usize, usize)> {
        let line = usize::try_from(position.line).ok()?;
        let column = positions::to_chars(self.lines.get(line)?, position.character)?;
        Some((line, column))
    }

    pub(crate) fn completion_prefix(&self) -> String {
        let prefix: String = self.line().chars().take(self.cursor_col).collect();
        let count = prefix
            .chars()
            .rev()
            .take_while(|ch| super::is_identifier_char(*ch))
            .count();
        prefix
            .chars()
            .skip(prefix.chars().count().saturating_sub(count))
            .collect()
    }

    fn identifier_end_position(&self) -> Option<Position> {
        let line = self.line();
        let suffix = line
            .chars()
            .skip(self.cursor_col)
            .take_while(|ch| super::is_identifier_char(*ch))
            .count();
        positions::from_chars(self.cursor_line, line, self.cursor_col + suffix)
    }

    fn fallback_edit(&self, item: &Completion, query: Query) -> TextEdit {
        let line = self.line();
        let count = self.completion_prefix().chars().count();
        let start = positions::from_chars(self.cursor_line, line, self.cursor_col - count)
            .unwrap_or(query.position);
        TextEdit {
            range: Range {
                start,
                end: query.position,
            },
            new_text: item.insert_text.clone(),
        }
    }
}

type Edits<'a> = Vec<(std::ops::Range<usize>, Cow<'a, str>)>;

fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Whether two edits, in start order, cannot both apply: the later one starts
/// inside the earlier, or where it starts, which leaves their order undefined.
fn clash(earlier: &std::ops::Range<usize>, later: &std::ops::Range<usize>) -> bool {
    let starts_inside = later.start < earlier.end;
    let starts_together = later.start == earlier.start;
    starts_inside || starts_together
}

fn prepare<'a>(
    text: &str,
    primary: &'a TextEdit,
    extra: &'a [TextEdit],
) -> Result<(Edits<'a>, usize), &'static str> {
    let primary_range =
        positions::byte_range(text, primary.range).ok_or("Invalid completion range")?;
    let mut edits = std::iter::once(primary)
        .chain(extra)
        .map(|edit| {
            positions::byte_range(text, edit.range)
                .map(|range| (range, normalize(&edit.new_text)))
                .ok_or("Invalid completion range")
        })
        .collect::<Result<Vec<_>, _>>()?;
    edits.sort_unstable_by_key(|(range, _)| (range.start, range.end));
    if edits
        .iter()
        .zip(edits.iter().skip(1))
        .any(|((earlier, _), (later, _))| clash(earlier, later))
    {
        return Err("Overlapping completion edits");
    }
    let mut caret = primary_range.start + normalize(&primary.new_text).len();
    for (range, replacement) in &edits {
        if range.start < primary_range.start {
            caret = caret - range.len() + replacement.len();
        }
    }
    Ok((edits, caret))
}
