//! Purpose: Validate and atomically apply Unicode completion edits to the editor buffer.

use super::Editor;
use crate::lsp::{
    features::{Completion, TextEdit},
    positions::{self, Range},
    types::Query,
};
use std::borrow::Cow;

impl Editor {
    pub(crate) fn complete_lsp(
        &mut self,
        query: Query,
        item: &Completion,
    ) -> Result<(), &'static str> {
        if self.lsp_stamp() != query.stamp || self.lsp_position() != Some(query.position) {
            return Err("Completion expired; request it again");
        }
        if item.snippet {
            return Err("Snippet completion is not supported");
        }
        let mut text = self.lsp_text();
        let primary = item
            .edit
            .clone()
            .unwrap_or_else(|| self.fallback_edit(item, query));
        let (edits, caret) = prepare(&text, &primary, &item.additional_edits)?;
        for (range, replacement) in edits.into_iter().rev() {
            text.replace_range(range, &replacement);
        }
        let position = positions::at_byte(&text, caret).ok_or("Invalid completion cursor")?;
        self.set_completed_text(&text, caret);
        self.cursor_line = position.line as usize;
        self.cursor_col = positions::to_chars(&self.lines[self.cursor_line], position.character)
            .ok_or("Invalid completion cursor")?;
        self.selection = super::super::EditorSelection::None;
        self.mark_modified();
        Ok(())
    }

    fn set_completed_text(&mut self, text: &str, caret: usize) {
        // Keep the final empty line only when the inserted text places the caret there.
        self.format.trailing_newline = text.ends_with('\n') && caret < text.len();
        let content = if self.format.trailing_newline {
            &text[..text.len() - 1]
        } else {
            text
        };
        self.lines = content.split('\n').map(str::to_owned).collect();
    }

    fn fallback_edit(&self, item: &Completion, query: Query) -> TextEdit {
        let line = &self.lines[self.cursor_line];
        let prefix: String = line.chars().take(self.cursor_col).collect();
        let count = prefix
            .chars()
            .rev()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
            .count();
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
        .windows(2)
        .any(|pair| pair[0].0.end > pair[1].0.start || pair[0].0.start == pair[1].0.start)
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
