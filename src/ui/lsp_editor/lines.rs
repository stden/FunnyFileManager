//! Purpose: The Language servers dialog's one-line rows: the settings row
//! naming the file F2 writes (or why there is none) and the message row
//! with the last notice or what changed, each cut to its width.

use std::collections::BTreeSet;

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::super::{fit_path, head_cells, palette, Cells};
use super::window::DetailPane;
use crate::config::LanguageId;
use crate::models::lsp_editor::{Access, Notice, ReadOnlyReason};
use crate::models::LspEditor;

/// What row 1 starts with.
const SETTINGS_CAPTION: &str = "Settings: ";

/// Row 1: the file F2 writes, or why there is none.
///
/// A path too long for the row gives up its head first, keeping at least
/// the file's name, and only then is the row cut at its end: the name and
/// the reason are what the row is for.
pub(super) fn settings_line(editor: &LspEditor, width: u16) -> Line<'static> {
    let failed = Style::default().fg(palette::STATUS_FAILED);
    let (file, after) = match editor.access() {
        Access::Writable { path } => (Some(path), None),
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile) => (
            None,
            Some(Span::styled("none for this session — read only", failed)),
        ),
        Access::ReadOnly(reason @ ReadOnlyReason::Document { path, .. }) => (
            Some(path),
            Some(Span::styled(format!(" — read only: {reason}"), failed)),
        ),
    };
    let words = Cells::of(SETTINGS_CAPTION)
        + after
            .as_ref()
            .map_or(Cells::ZERO, |span| Cells::of(&span.content));
    let path = file.map(|file| {
        Span::styled(
            fit_path(&file.display().to_string(), words, Cells::columns(width)),
            super::super::code::token_style(crate::highlight::Token::Str),
        )
    });
    let spans = std::iter::once(Span::raw(SETTINGS_CAPTION))
        .chain(path)
        .chain(after)
        .collect();
    fit_line(spans, width)
}

/// The message row: the notice the last key set, else why the dialog is
/// read-only, else what changed.
pub(super) fn message_line(editor: &LspEditor, pane: DetailPane, width: u16) -> Line<'static> {
    let notice = editor
        .notice()
        .cloned()
        .or_else(|| editor.read_only_notice());
    let (text, color) = match (notice, pane) {
        (Some(Notice::Info(text)), _) => (text, palette::CODE_NORMAL),
        (Some(Notice::Error(text)), _) => (text, palette::STATUS_FAILED),
        (None, DetailPane::Hidden) => ("F3 shows details".to_owned(), palette::CODE_NORMAL),
        (None, DetailPane::Shown) => (changes(editor), palette::CODE_NORMAL),
    };
    fit_line(vec![Span::styled(text, Style::default().fg(color))], width)
}

/// `Modified: python, rust. F2 saves only what changed.`, or `No changes`.
fn changes(editor: &LspEditor) -> String {
    let changed: BTreeSet<&str> = editor
        .draft()
        .0
        .keys()
        .chain(editor.baseline().0.keys())
        .map(String::as_str)
        .filter(|key| editor.is_changed(&LanguageId::from_key(key)))
        .collect();
    if changed.is_empty() {
        return "No changes".to_owned();
    }
    let changed: Vec<&str> = changed.into_iter().collect();
    format!(
        "Modified: {}. F2 saves only what changed.",
        changed.join(", ")
    )
}

/// `spans` cut to `width` cells, the last one marked `…` when cut.
pub(super) fn fit_line(spans: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let mut room = Cells::columns(width);
    let mut kept = Vec::new();
    for span in spans {
        let cells = Cells::of(&span.content);
        if cells <= room {
            room = room.saturating_sub(cells);
            kept.push(span);
            continue;
        }
        kept.push(Span::styled(head_cells(&span.content, room), span.style));
        break;
    }
    Line::from(kept)
}
