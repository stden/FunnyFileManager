//! Purpose: Validate and atomically apply Unicode completion edits to the editor buffer.

mod prepare;

use self::prepare::{normalize, prepare};
use super::Editor;
use crate::lsp::{
    features::{Completion, TextEdit},
    positions::{self, Position, Range},
    types::Query,
};

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
        let primary = self.primary_edit(item, query, overwrite_suffix);
        if let Some(outcome) = self.try_single_line(&primary, &item.additional_edits) {
            return outcome;
        }
        let mut text = self.lsp_text();
        let (edits, caret) = prepare(&text, &primary, &item.additional_edits)?;
        for (range, replacement) in edits.into_iter().rev() {
            text.replace_range(range, &replacement);
        }
        let position = positions::at_byte(&text, caret).ok_or("Invalid completion cursor")?;
        self.set_completed_text(&text, caret);
        let (line, column) = self.caret_at(position).ok_or("Invalid completion cursor")?;
        self.finish_completion(line, column);
        Ok(())
    }

    fn try_single_line(
        &mut self,
        primary: &TextEdit,
        extra: &[TextEdit],
    ) -> Option<Result<(), &'static str>> {
        if !extra.is_empty() || primary.range.start.line != primary.range.end.line {
            return None;
        }
        let replacement = normalize(&primary.new_text);
        if replacement.contains('\n') || replacement.contains('\r') {
            return None;
        }
        let line_idx = usize::try_from(primary.range.start.line).ok()?;
        if self.lines.get(line_idx).is_none_or(|l| l.ends_with('\r')) {
            return None;
        }
        Some(self.edit_line_in_place(line_idx, primary.range, &replacement))
    }

    fn edit_line_in_place(
        &mut self,
        line_idx: usize,
        range: Range,
        replacement: &str,
    ) -> Result<(), &'static str> {
        let line = self
            .lines
            .get_mut(line_idx)
            .ok_or("Invalid completion range")?;
        let start = positions::column_byte(line, range.start.character)
            .ok_or("Invalid completion range")?;
        let end =
            positions::column_byte(line, range.end.character).ok_or("Invalid completion range")?;
        if start > end {
            return Err("Invalid completion range");
        }
        line.replace_range(start..end, replacement);
        let column = line
            .get(..start + replacement.len())
            .ok_or("Invalid completion cursor")?
            .chars()
            .count();
        self.note_dirty(line_idx, 1);
        self.finish_completion(line_idx, column);
        Ok(())
    }

    fn finish_completion(&mut self, line: usize, column: usize) {
        self.cursor_line = line;
        self.cursor_col = column;
        self.selection = super::super::EditorSelection::None;
        self.lsp.completions = None;
        self.mark_modified();
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
        self.note_dirty(0, self.lines.len().max(1));
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
