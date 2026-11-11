//! Purpose: Validate and atomically apply Unicode completion edits to the editor buffer.

mod prepare;
mod splice;

use self::prepare::prepare_groups;
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
        if let Some(line_idx) = self.single_line_index(&primary, &item.additional_edits) {
            return self.edit_line_in_place(line_idx, primary.range, &primary.new_text);
        }
        let groups = prepare_groups(
            &self.lines,
            self.format.trailing_newline,
            &primary,
            &item.additional_edits,
        )?;
        let caret = self.splice_completion_groups(&groups);
        self.finish_completion(caret.line, caret.column);
        Ok(())
    }

    fn single_line_index(&self, primary: &TextEdit, extra: &[TextEdit]) -> Option<usize> {
        if !extra.is_empty()
            || primary.range.start.line != primary.range.end.line
            || primary.new_text.contains(['\n', '\r'])
        {
            return None;
        }
        let line_idx = usize::try_from(primary.range.start.line).ok()?;
        self.lines
            .get(line_idx)
            .filter(|line| !line.ends_with('\r'))
            .map(|_| line_idx)
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
        self.note_dirty(line_idx, line_idx + 1);
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

    pub(super) fn completion_prefix_str(&self) -> &str {
        let before_cursor = self
            .line()
            .get(..self.byte_col(self.cursor_col))
            .unwrap_or_default();
        let head = before_cursor.trim_end_matches(super::is_identifier_char);
        before_cursor.get(head.len()..).unwrap_or_default()
    }

    pub(crate) fn completion_prefix(&self) -> String {
        self.completion_prefix_str().to_owned()
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
        let count = self.completion_prefix_str().chars().count();
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
