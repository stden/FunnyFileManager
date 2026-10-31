//! Purpose: The columns of a server row in the Language servers list: their
//! widths, which give way on a narrow screen, and each column's pieces — the
//! marks, the checkbox, the command and extensions, the availability tag.

use ratatui::style::Style;

use super::super::super::{head_cells, palette, Cells};
use super::super::column;
use super::super::detail::{conflicts, extension_pieces, impact_style, tone_style, Piece};
use crate::config::{Enablement, LanguageId, ServerSettings};
use crate::models::lsp_editor::Impact;
use crate::models::LspEditor;

/// The fixed columns: marker, language, marks, checkbox, availability tag.
const MARKER: usize = 2;
pub(super) const LANGUAGE: usize = 16;
const MARKS: usize = 2;
const CHECKBOX: usize = 3;
const TAG: usize = 7;
/// Blanks between the columns: after the language, the marks, the checkbox,
/// the command and the extensions.
const GAPS: usize = 5;
/// The extensions column at full width, and the least the command keeps
/// before the extensions start to give way.
const EXTENSIONS: usize = 15;
const MIN_COMMAND: usize = 8;

/// The widths of the two columns that give way on a narrow screen: the
/// command shrinks first, then the extensions.
fn widths(row: usize) -> (usize, usize) {
    let fixed = MARKER + LANGUAGE + MARKS + CHECKBOX + TAG + GAPS;
    let rest = row.saturating_sub(fixed);
    let extensions = rest.saturating_sub(MIN_COMMAND).min(EXTENSIONS);
    (rest - extensions, extensions)
}

/// The command and extensions columns, dimmed for a disabled server.
pub(super) fn program(
    editor: &LspEditor,
    language: &LanguageId,
    server: &ServerSettings,
    width: usize,
) -> Vec<Piece> {
    let (command_width, extensions_width) = widths(width);
    let enabled = server.enabled == Enablement::Enabled;
    let dim = |style: Style| {
        if enabled {
            style
        } else {
            Style::default().fg(palette::DISABLED_FG)
        }
    };
    let command = column(&server.command, command_width);
    let mut pieces = vec![
        (command, dim(Style::default().fg(palette::DIRECTORY))),
        (" ".to_owned(), Style::default()),
    ];
    let extensions = extension_pieces(server, &conflicts(editor, language));
    pieces.extend(fit(extensions, extensions_width, dim));
    pieces
}

/// `*` when the server differs from the file, `!` when it has a problem.
pub(super) fn marks(editor: &LspEditor, language: &LanguageId) -> [Piece; 2] {
    let changed = if editor.is_changed(language) {
        ("*".to_owned(), Style::default().fg(palette::MARKED))
    } else {
        (" ".to_owned(), Style::default())
    };
    let impact = editor
        .problems()
        .into_iter()
        .filter(|(problem, _)| problem.involves(language))
        .map(|(_, impact)| impact)
        .min_by_key(|impact| match impact {
            Impact::Blocking => 0,
            Impact::Warning => 1,
        });
    let problem = impact.map_or_else(
        || (" ".to_owned(), Style::default()),
        |impact| ("!".to_owned(), impact_style(impact)),
    );
    [changed, problem]
}

pub(super) fn checkbox(enabled: Enablement) -> Piece {
    match enabled {
        Enablement::Enabled => ("[x]".to_owned(), Style::default().fg(palette::STATUS_OK)),
        Enablement::Disabled => ("[ ]".to_owned(), Style::default().fg(palette::DISABLED_FG)),
    }
}

/// The availability tag, right-aligned in its column.
pub(super) fn tag(editor: &LspEditor, language: &LanguageId) -> Piece {
    editor.availability(language).map_or_else(
        || (" ".repeat(TAG), Style::default()),
        |availability| {
            (
                format!("{:>TAG$}", availability.tag()),
                tone_style(availability.tone()),
            )
        },
    )
}

/// `pieces` cut to `width` cells, the cut one ending in `…`, padded with
/// blanks; `style` restyles each piece (to dim a disabled server).
fn fit(pieces: Vec<Piece>, width: usize, style: impl Fn(Style) -> Style) -> Vec<Piece> {
    let mut room = Cells::new(width);
    let mut fitted = Vec::new();
    for (text, own) in pieces {
        let cells = Cells::of(&text);
        if cells > room {
            let cut = head_cells(&text, room);
            room = room.saturating_sub(Cells::of(&cut));
            fitted.push((cut, style(own)));
            break;
        }
        room = room.saturating_sub(cells);
        fitted.push((text, style(own)));
    }
    fitted.push((" ".repeat(room.get()), Style::default()));
    fitted
}

#[cfg(test)]
mod tests_tag;
