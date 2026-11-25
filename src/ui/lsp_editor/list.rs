//! Purpose: The server list of the Language servers dialog: one row per
//! server, its columns, marks and colours, and the legend its keys give.

mod columns;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{code::token_style, palette};
use super::detail::Piece;
use super::{column, ground};
use crate::config::{LanguageId, ServerSettings};
use crate::highlight::Token;
use crate::models::lsp_editor::{Access, Origin};
use crate::models::LspEditor;
use columns::{checkbox, marks, program, tag, LANGUAGE};

/// Whether a row is the selected one, drawn in the cursor's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowFocus {
    Selected,
    Other,
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
