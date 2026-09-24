//! Purpose: Draws Far's `F11` Plugin commands list — a menu on the menu's
//! teal, one row per command, the cursor row inverted.

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::drives::centered;
use super::panel::pad_cells;
use super::{dialog, far_block, palette, Cells};
use crate::models::{PluginCommand, PluginCommands};

/// Written into the top border, as Far titles it.
const TITLE: &str = " Plugin commands ";

/// Blank columns either side of a row's text.
const PAD: usize = 1;
/// The cells the double frame takes, across and down.
const FRAME: usize = 2;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    list: &PluginCommands,
) {
    let labels = PluginCommand::ALL.map(PluginCommand::label);
    let text_width = labels
        .iter()
        .map(|label| Cells::of(label).get())
        .max()
        .unwrap_or(0)
        .max(Cells::of(TITLE).get());
    // The rows, a blank column either side of them, and the frame's two.
    let width = PAD + text_width + PAD + FRAME;
    let area = centered(f.area(), width, labels.len() + FRAME);
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::MENU_FG);

    dialog::render_shadow(f, area, f.area());
    f.render_widget(Clear, area);
    let block = far_block()
        .title(Line::from(TITLE).centered())
        .border_style(ground)
        .style(ground);
    let body = block.inner(area);
    f.render_widget(block, area);
    let lines: Vec<Line<'_>> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| row(label, text_width, index == list.cursor(), ground))
        .collect();
    f.render_widget(Paragraph::new(lines), body);

    // Only the frame is recorded: a click anywhere on it is a drag handle.
    tracker.area = Some(area);
    tracker.body = None;
    tracker.buttons.clear();
}

fn row(label: &str, text_width: usize, chosen: bool, ground: Style) -> Line<'static> {
    let style = if chosen {
        Style::default()
            .bg(palette::CURSOR_BG)
            .fg(palette::CURSOR_FG)
    } else {
        ground
    };
    let padded = pad_cells(label, Cells::new(text_width));
    Line::from(Span::styled(
        format!("{0}{padded}{0}", " ".repeat(PAD)),
        style,
    ))
}
