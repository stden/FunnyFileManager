//! Purpose: The F-key bar on the bottom row, with twelve labels spread edge to edge.
//!
//! The panels pass Far's own labels; a full-screen view with keys of its own
//! (the task list) passes its set. Either way the cells are laid out by the
//! same arithmetic the mouse hit test uses.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{fit_name, palette, take_cells, Cells};

/// How many cells the key bar has, which the hit test needs too.
///
/// Twelve, not ten: F11 and F12 are part of the bar Far draws, and a bar that
/// stops at ten tells the user those keys do nothing.
pub const FKEY_COUNT: usize = 12;

/// The twelve F-key cells, in Far's order and with Far's abbreviations.
pub(super) const FKEY_LABELS: [(&str, &str); FKEY_COUNT] = [
    ("1", "Help"),
    ("2", "UserMn"),
    ("3", "View"),
    ("4", "Edit"),
    ("5", "Copy"),
    ("6", "RenMov"),
    ("7", "MkFold"),
    ("8", "Delete"),
    ("9", "ConfMn"),
    ("10", "Quit"),
    // Far's F11 lists plugin commands: here the MCP status screen and
    // `NetBox`'s stored sessions and their import.
    ("11", "Plugin"),
    // Far's F12 switches between open screens. There are two here — the panels
    // and the console behind them — and F12 moves between them.
    ("12", "Screen"),
];

/// One cell of the key bar: the key's number and its label, empty when the
/// key does nothing on the current screen.
pub(super) type KeyLabels<'a> = [(&'a str, &'a str); FKEY_COUNT];

/// Draws `labels`, each a key number and its label, as Far's key bar.
///
/// Exactly [`FKEY_COUNT`] of them: the mouse hit test resolves a click against
/// that many cells, so a bar drawn with any other number would put a label
/// where a click runs a different key.
///
/// Far's key bar fills the row: every cell is an equal share of the terminal,
/// so the coloured blocks meet edge to edge instead of trailing off into
/// background after the last label.
///
/// The remainder is spread over the leftmost cells rather than dumped on the
/// last one, which is what keeps an 85-column terminal from ending in a cell
/// five columns wider than its neighbours.
///
/// A screen with keys of its own (the Language servers dialog) passes its own
/// labels; a label for a key that does nothing there is left empty, because a
/// label that promises nothing is worse than none.
pub(super) fn render_key_bar(f: &mut Frame<'_>, area: Rect, labels: &KeyLabels<'_>) {
    // The same arithmetic the hit test uses, so a click that lands on "5Copy"
    // runs F5 because one function put the label there and answered for it.
    let widths = crate::mouse::fkey_widths(area.width, labels.len());

    let spans: Vec<Span<'_>> = labels
        .iter()
        .zip(widths)
        .flat_map(|((key, label), width)| {
            // The key number is clipped to its cell like the label is: a
            // two-digit key in a one-column cell would otherwise push every
            // cell after it one column right of where `fkey_at` expects it,
            // and a click on the visible "10" would run F11.
            let cell = Cells::new(usize::from(width));
            let (key, key_width) = take_cells(key, cell);
            let label_width = cell.saturating_sub(key_width);
            vec![
                Span::styled(
                    key,
                    Style::default().bg(Color::Black).fg(palette::DIRECTORY),
                ),
                Span::styled(
                    fit_name(label, label_width),
                    Style::default()
                        .bg(palette::CURSOR_BG)
                        .fg(palette::HIGHLIGHT_FG),
                ),
            ]
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Black).fg(palette::HIGHLIGHT_FG))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}
