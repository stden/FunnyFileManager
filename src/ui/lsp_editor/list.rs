//! Purpose: The server list of the Language servers dialog: one row per
//! server, its columns, marks and colours, and the legend its keys give.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{code::token_style, palette, Cells};
use super::detail::{conflicts, extension_pieces, impact_style, tone_style, Piece};
use super::{column, ground};
use crate::config::{Enablement, LanguageId, ServerSettings};
use crate::highlight::Token;
use crate::models::lsp_editor::{Access, Impact, Origin};
use crate::models::LspEditor;

/// Whether a row is the selected one, drawn in the cursor's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowFocus {
    Selected,
    Other,
}

/// The fixed columns: marker, language, marks, checkbox, availability tag.
const MARKER: usize = 2;
const LANGUAGE: usize = 16;
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

/// Whether Del removes the selected server rather than resetting it.
pub(super) fn removes(editor: &LspEditor) -> bool {
    editor
        .selected()
        .is_some_and(|language| LspEditor::origin(language) == Origin::Added)
}

/// The keys that are not F-keys, written into the bottom border.
pub(super) fn legend(editor: &LspEditor) -> String {
    if matches!(editor.access(), Access::ReadOnly(_)) {
        return "F3 View  F5 Check  Esc Close".to_owned();
    }
    let delete = if removes(editor) { "Remove" } else { "Reset" };
    format!("Enter Edit  Space On/Off  Ins Add  Del {delete}  Esc Close")
}

/// The rows that fit in `area`, scrolled to keep the selection in view.
pub(super) fn render(f: &mut Frame<'_>, area: Rect, editor: &LspEditor) {
    let height = usize::from(area.height);
    if height == 0 {
        return;
    }
    let selected = editor.selected_index().unwrap_or(0);
    let first = selected.saturating_sub(height - 1);
    let width = usize::from(area.width);
    let lines: Vec<Line<'_>> = editor
        .draft()
        .0
        .iter()
        .enumerate()
        .skip(first)
        .take(height)
        .map(|(index, (key, server))| {
            let focus = if index == selected {
                RowFocus::Selected
            } else {
                RowFocus::Other
            };
            row(editor, &LanguageId::from_key(key), server, width, focus)
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(ground()), area);
}

/// One server's row. The selected row is drawn uniformly in the cursor's
/// colours, as a panel's is: per-cell colours would clash with its ground.
fn row(
    editor: &LspEditor,
    language: &LanguageId,
    server: &ServerSettings,
    width: usize,
    focus: RowFocus,
) -> Line<'static> {
    let pieces = pieces(editor, language, server, width, focus);
    let spans = pieces.into_iter().map(|(text, style)| {
        if focus == RowFocus::Selected {
            Span::styled(
                text,
                Style::default()
                    .fg(palette::CURSOR_FG)
                    .bg(palette::CURSOR_BG),
            )
        } else {
            Span::styled(text, style)
        }
    });
    Line::from(spans.collect::<Vec<_>>())
}

fn pieces(
    editor: &LspEditor,
    language: &LanguageId,
    server: &ServerSettings,
    width: usize,
    focus: RowFocus,
) -> Vec<Piece> {
    let marker = match focus {
        RowFocus::Selected => "▸ ",
        RowFocus::Other => "  ",
    };
    let mut pieces = vec![
        (marker.to_owned(), Style::default()),
        (
            column(language.as_str(), LANGUAGE),
            token_style(Token::Keyword),
        ),
        (" ".to_owned(), Style::default()),
    ];
    pieces.extend(marks(editor, language));
    pieces.push((" ".to_owned(), Style::default()));
    pieces.push(checkbox(server.enabled));
    pieces.push((" ".to_owned(), Style::default()));
    pieces.extend(program(editor, language, server, width));
    pieces.push((" ".to_owned(), Style::default()));
    pieces.push(tag(editor, language));
    pieces
}

/// The command and extensions columns, dimmed for a disabled server.
fn program(
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
fn marks(editor: &LspEditor, language: &LanguageId) -> [Piece; 2] {
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

fn checkbox(enabled: Enablement) -> Piece {
    match enabled {
        Enablement::Enabled => ("[x]".to_owned(), Style::default().fg(palette::STATUS_OK)),
        Enablement::Disabled => ("[ ]".to_owned(), Style::default().fg(palette::DISABLED_FG)),
    }
}

/// The availability tag, right-aligned in its column.
fn tag(editor: &LspEditor, language: &LanguageId) -> Piece {
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
            let cut = super::head_cells(&text, room);
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
