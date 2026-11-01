//! Purpose: Draws the Archive commands menu (Shift+F3) — Far's rows as a boxed
//! list, the one under the cursor inverted, each with the letter that runs it.
//!
//! A menu rather than a form, as Far's plugin draws it: the only question is
//! "which command", and a list with a cursor answers that in one keystroke.
//! Drawn like the drive menu — the same teal ground, the same hotkey column —
//! so the two lists the panels open read as one family.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::panel::pad_cells;
use super::{palette, Cells};
use crate::models::{ArchiveCommand, ArchiveMenu};

/// Written into the top border, as Far titles the plugin's menu.
const TITLE: &str = " Archive commands ";

/// Written into the bottom border. Only the keys the menu actually handles:
/// a legend promising keys that do nothing is worse than no legend.
const FOOTER: &str = " Enter  Esc ";

/// `" T "` ahead of a row's label — the drive menu's hotkey column.
const HOTKEY_CELLS: usize = 3;

/// The outer frame's column on each side, and the column of ground Far leaves
/// between the two frames, twice over.
const FRAME_COLUMNS: usize = 6;

/// The outer frame's top and bottom rows.
const FRAME_ROWS: usize = 2;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &ArchiveMenu,
) {
    let interior = interior_width();
    let lines = body(interior, dialog.command());
    let area = placed(
        f.area(),
        interior + FRAME_COLUMNS,
        lines.len() + FRAME_ROWS,
        tracker.offset,
    );
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::DRIVE_FG);

    let block = super::dialog_frame(TITLE, FOOTER, ground);

    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
    track(tracker, area);
}

/// Record where the menu landed, so a click can be told from a drag.
///
/// The body is the rows inside the inner box, with the two rows the tracker
/// discounts as rule-and-buttons standing in for the box's bottom edge and
/// the frame under it; there are no buttons, so the button row is the frame's
/// bottom border, which the tracker already treats as chrome.
fn track(tracker: &mut crate::mouse::DialogMouse, area: Rect) {
    let rows = u16::try_from(ArchiveCommand::ALL.len()).unwrap_or(u16::MAX);
    tracker.area = Some(area);
    tracker.body = Some(Rect {
        x: area.x + 3,
        y: area.y + 2,
        width: area.width.saturating_sub(6),
        height: rows + 2,
    });
    tracker.buttons.clear();
    tracker.buttons_y = area.bottom().saturating_sub(1);
}

/// The inner box's interior: the widest row, or the footer if that is wider.
fn interior_width() -> usize {
    ArchiveCommand::ALL
        .iter()
        .map(|command| Cells::of(command.label()).get() + HOTKEY_CELLS)
        .chain(std::iter::once(Cells::of(FOOTER).get()))
        .max()
        .unwrap_or_default()
}

/// The lines inside the outer frame: the single-line box Far draws around the
/// rows, one row per command.
fn body(interior: usize, chosen: ArchiveCommand) -> Vec<Line<'static>> {
    let edge = |left: char, right: char| {
        Line::from(Span::styled(
            format!("{left}{}{right}", "─".repeat(interior)),
            Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG),
        ))
    };
    let mut lines = Vec::with_capacity(ArchiveCommand::ALL.len() + 2);
    lines.push(edge('┌', '┐'));
    lines.extend(
        ArchiveCommand::ALL
            .iter()
            .map(|command| row_line(*command, interior, *command == chosen)),
    );
    lines.push(edge('└', '┘'));
    lines
}

/// One row: the hotkey in Far's yellow, then the label out to the box's edge.
///
/// The filler belongs to the label's span, not the frame's, so the selection
/// bar runs the full width of the box instead of stopping at the last letter.
fn row_line(command: ArchiveCommand, interior: usize, chosen: bool) -> Line<'static> {
    let background = if chosen {
        palette::MENU_SELECTED_BG
    } else {
        palette::MENU_BG
    };
    let frame = Style::default().fg(palette::DRIVE_FG).bg(palette::MENU_BG);
    let hotkey = palette::hotkey_style(Style::default().bg(background));
    let label = Style::default()
        .fg(if chosen {
            palette::MENU_SELECTED_FG
        } else {
            palette::DRIVE_FG
        })
        .bg(background);
    Line::from(vec![
        Span::styled("│", frame),
        Span::styled(
            format!(" {} ", command.hotkey().to_ascii_uppercase()),
            hotkey,
        ),
        Span::styled(
            pad_cells(
                command.label(),
                Cells::new(interior.saturating_sub(HOTKEY_CELLS)),
            ),
            label,
        ),
        Span::styled("│", frame),
    ])
}

/// Centred on the screen, then shifted by however far the user has dragged
/// it — clamped, so the menu can never be dragged off the screen and lost.
fn placed(screen: Rect, width: usize, height: usize, offset: (i32, i32)) -> Rect {
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(screen.width);
    let height = u16::try_from(height).unwrap_or(u16::MAX).min(screen.height);
    let centred_x = i32::from(screen.x) + i32::from((screen.width - width) / 2);
    let centred_y = i32::from(screen.y) + i32::from((screen.height - height) / 2);
    let max_x = i32::from(screen.right() - width);
    let max_y = i32::from(screen.bottom() - height);
    Rect {
        x: u16::try_from((centred_x + offset.0).clamp(i32::from(screen.x), max_x)).unwrap_or(0),
        y: u16::try_from((centred_y + offset.1).clamp(i32::from(screen.y), max_y)).unwrap_or(0),
        width,
        height,
    }
}

#[cfg(test)]
mod tests;
