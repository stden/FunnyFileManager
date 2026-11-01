//! Purpose: One server's detail — the rows the list's lower pane and the F3
//! view both draw — built from typed data, coloured, and wrapped.
//!
//! Drawn from `ServerSettings` and `Availability`, never from report text, so
//! the colours cannot drift from the wording. The labels are written here and
//! nowhere else; the status words belong to `Availability::label`.

mod values;

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

use super::super::{code::token_style, palette, Cells};
use crate::config::{Enablement, LanguageId, ServerSettings};
use crate::highlight::Token;
use crate::lsp::Tone;
use crate::models::lsp_editor::{Impact, Problem};
use crate::models::LspEditor;
use values::rows;

pub(super) const COMMAND: &str = "Command:";
pub(super) const ARGUMENTS: &str = "Arguments:";
pub(super) const STATUS: &str = "Status:";
pub(super) const EXECUTABLE: &str = "Executable:";
pub(super) const LAUNCH_ARGUMENTS: &str = "Launch arguments:";
pub(super) const AUTO_INSTALL: &str = "Auto-install:";
pub(super) const PROBLEM: &str = "Problem:";

/// Where a value starts: two blanks, the longest label and one blank.
const VALUE_COLUMN: usize = 2 + LAUNCH_ARGUMENTS.len() + 1;

/// A run of text and its colour.
pub(super) type Piece = (String, Style);

fn plain(color: ratatui::style::Color) -> Style {
    Style::default().fg(color)
}

/// The colour a status has, by how it reads at a glance.
pub(super) fn tone_style(tone: Tone) -> Style {
    plain(match tone {
        Tone::Good => palette::STATUS_OK,
        Tone::Pending => palette::STATUS_PENDING,
        Tone::Bad => palette::STATUS_FAILED,
    })
}

pub(super) fn impact_style(impact: Impact) -> Style {
    plain(match impact {
        Impact::Blocking => palette::STATUS_FAILED,
        Impact::Warning => palette::STATUS_PENDING,
    })
}

/// The extensions two enabled servers claim, `language` among them, in lower
/// case.
pub(super) fn conflicts(editor: &LspEditor, language: &LanguageId) -> Vec<String> {
    editor
        .problems()
        .into_iter()
        .filter(|(problem, _)| problem.involves(language))
        .filter_map(|(problem, _)| {
            if let Problem::DuplicateExtension { extension, .. } = problem {
                Some(extension)
            } else {
                None
            }
        })
        .collect()
}

/// `.py .pyi`, each extension in the type colour, or red when another
/// enabled server claims it too.
pub(super) fn extension_pieces(server: &ServerSettings, conflicts: &[String]) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for (index, extension) in server.extensions.iter().enumerate() {
        if index > 0 {
            pieces.push((" ".to_owned(), Style::default()));
        }
        let style = if conflicts.contains(&extension.to_ascii_lowercase()) {
            plain(palette::STATUS_FAILED)
        } else {
            token_style(Token::Type)
        };
        pieces.push((format!(".{extension}"), style));
    }
    pieces
}

/// `[enabled]` or `[disabled]`.
fn enablement_piece(enabled: Enablement) -> Piece {
    match enabled {
        Enablement::Enabled => ("[enabled]".to_owned(), plain(palette::STATUS_OK)),
        Enablement::Disabled => ("[disabled]".to_owned(), plain(palette::DISABLED_FG)),
    }
}

/// Every row of `language`'s detail, wrapped to `width`.
pub(super) fn detail_lines(
    editor: &LspEditor,
    language: &LanguageId,
    server: &ServerSettings,
    width: Cells,
) -> Vec<Line<'static>> {
    let conflicts = conflicts(editor, language);
    let mut heading = vec![
        (language.as_str().to_owned(), token_style(Token::Keyword)),
        ("  ".to_owned(), Style::default()),
        enablement_piece(server.enabled),
        ("  ".to_owned(), Style::default()),
    ];
    heading.extend(extension_pieces(server, &conflicts));
    let mut lines = wrap(heading, 2, width);
    for (label, value) in rows(editor, language, server) {
        lines.extend(labelled(label, value, width));
    }
    lines
}

/// `label` and its value, the value wrapped under itself.
fn labelled(label: &str, value: Vec<Piece>, width: Cells) -> Vec<Line<'static>> {
    let mut pieces = vec![(
        format!("  {label:<width$}", width = VALUE_COLUMN - 2),
        plain(palette::CODE_NORMAL),
    )];
    pieces.extend(value);
    wrap(pieces, VALUE_COLUMN, width)
}

/// `pieces` laid out in lines of `width` cells, each line after the first
/// indented by `indent`. A piece split across lines keeps its colour.
pub(super) fn wrap(pieces: Vec<Piece>, indent: usize, width: Cells) -> Vec<Line<'static>> {
    let width = width.get().max(indent + 1);
    let mut lines = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for (text, style) in pieces {
        let mut run = String::new();
        for ch in text.chars() {
            let cells = ch.width().unwrap_or(0);
            if used + cells > width && used > indent {
                spans.push(Span::styled(std::mem::take(&mut run), style));
                lines.push(Line::from(std::mem::take(&mut spans)));
                spans.push(Span::raw(" ".repeat(indent)));
                used = indent;
            }
            run.push(ch);
            used += cells;
        }
        spans.push(Span::styled(run, style));
    }
    lines.push(Line::from(spans));
    lines
}

/// The F3 view: every server's detail, a blank line after each.
pub(super) fn every_server(editor: &LspEditor, width: Cells) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (key, server) in &editor.draft().0 {
        lines.extend(detail_lines(
            editor,
            &LanguageId::from_key(key),
            server,
            width,
        ));
        lines.push(Line::default());
    }
    lines
}

#[cfg(test)]
mod tests_conflicts;
