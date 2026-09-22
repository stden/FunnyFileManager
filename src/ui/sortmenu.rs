//! Purpose: Draws the Sort by menu (Ctrl+F12) as Far draws it — a drop-down
//! over the panel it sorts, on the menu's teal, with a tick against the mode
//! in force, not a grey form.
//!
//! The rows are the ones Far's help lists, each with its `Ctrl+F` key pushed
//! to the right edge so the eye can run down the column of keys, and under a
//! rule the order switch, ticked while the panel runs the other way. What is
//! ticked comes from the menu itself: it took a snapshot of the panel when it
//! opened, and every choice closes it, so the snapshot never lies.
//!
//! On a terminal too short for every row the list scrolls so the cursor row
//! is always drawn: what the keyboard can reach, the eye can see.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{far_block, palette, Cells};
use crate::models::{PanelSide, SortMenu, SortRow};

/// Written into the top border, as Far titles it.
const TITLE: &str = " Sort by ";

/// Blank columns inside the frame, left of the tick and right of the key.
const PAD: usize = 1;

/// Far's tick, against the mode in force and against a reversed order.
pub(super) const TICK: char = '√';

/// The least room between the longest label and the column of keys.
const GAP: usize = 2;

/// How far in from the panel's left edge the box drops, as Far's does.
const INSET: u16 = 4;

/// Draw the menu and record where it landed, for a drag.
pub(super) fn render(f: &mut Frame, tracker: &mut crate::mouse::DialogMouse, dialog: &SortMenu) {
    let Some(area) = place(f.area(), dialog.side, tracker.offset) else {
        tracker.clear();
        return;
    };
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::MENU_FG);
    let body = render_frame(f, area, ground);
    render_lines(f, dialog, body, ground);

    // Only the frame is recorded: a click on a row is answered by `row_at`,
    // from the same geometry, and a click anywhere else is a drag handle.
    tracker.area = Some(area);
    tracker.body = None;
    tracker.buttons.clear();
}

/// The shadow, the cleared box and the titled frame; returns the area inside it.
fn render_frame(f: &mut Frame, area: Rect, ground: Style) -> Rect {
    super::dialog::render_shadow(f, area, f.area());
    f.render_widget(Clear, area);
    let frame = far_block()
        .title(Line::from(TITLE).centered())
        .border_style(ground)
        .style(ground);
    let body = frame.inner(area);
    f.render_widget(frame, area);
    body
}

/// The rows and the rule, scrolled so the cursor row is always drawn.
fn render_lines(f: &mut Frame, dialog: &SortMenu, body: Rect, ground: Style) {
    let first = first_line(dialog, body.height);
    let shown = lines()
        .into_iter()
        .skip(first)
        .take(usize::from(body.height));
    for (offset, line) in shown.enumerate() {
        let Ok(offset) = u16::try_from(offset) else {
            break;
        };
        let line_area = Rect {
            y: body.y + offset,
            height: 1,
            ..body
        };
        match line {
            Some(row) => render_row(f, row, dialog, ground, line_area),
            None => render_rule(f, ground, line_area),
        }
    }
}

/// The row under a click, as a cursor position — the rule between the modes
/// and the order switch is drawn on a line of its own and names no row.
pub fn row_at(
    screen: Rect,
    menu: &SortMenu,
    offset: (i32, i32),
    column: u16,
    row: u16,
) -> Option<usize> {
    let body = far_block().inner(place(screen, menu.side, offset)?);
    if !body.contains((column, row).into()) {
        return None;
    }
    let line = first_line(menu, body.height) + usize::from(row - body.y);
    let clicked = lines().get(line).copied().flatten()?;
    SortRow::ALL.iter().position(|row| *row == clicked)
}

/// Where the box goes: sized to its rows, dropped over the panel it sorts a
/// little in from that panel's left edge, centred vertically, then shifted by
/// however far it has been dragged — clamped so it can never leave the screen.
///
/// `None` when the screen cannot hold even the frame.
pub(super) fn place(screen: Rect, side: PanelSide, offset: (i32, i32)) -> Option<Rect> {
    let width = u16::try_from(interior_width() + 2)
        .unwrap_or(u16::MAX)
        .min(screen.width);
    let height = u16::try_from(lines().len() + 2)
        .unwrap_or(u16::MAX)
        .min(screen.height);
    if width < 4 || height < 3 {
        return None;
    }
    let panel_left = match side {
        PanelSide::Left => 0,
        PanelSide::Right => screen.width / 2,
    };
    let anchored_x =
        i32::from(screen.x) + i32::from((panel_left + INSET).min(screen.width - width));
    let centred_y = i32::from(screen.y) + i32::from(screen.height - height) / 2;
    let max_x = i32::from(screen.right() - width);
    let max_y = i32::from(screen.bottom() - height);
    Some(Rect {
        x: u16::try_from((anchored_x + offset.0).clamp(i32::from(screen.x), max_x)).ok()?,
        y: u16::try_from((centred_y + offset.1).clamp(i32::from(screen.y), max_y)).ok()?,
        width,
        height,
    })
}

/// The menu top to bottom: every row, with the rule — `None` — between the
/// modes and the order switch.
fn lines() -> Vec<Option<SortRow>> {
    let mut lines: Vec<Option<SortRow>> = SortRow::ALL.iter().copied().map(Some).collect();
    lines.insert(SortRow::ALL.len() - 1, None);
    lines
}

/// The first line drawn: the list scrolls only as far as it must for the
/// cursor row to land on the body's last line.
fn first_line(menu: &SortMenu, body_height: u16) -> usize {
    let cursor = lines()
        .iter()
        .position(|line| *line == Some(menu.row()))
        .unwrap_or(0);
    (cursor + 1).saturating_sub(usize::from(body_height))
}

/// The columns inside the frame: the padding, the tick and its blank, the
/// widest label, the gap, and the widest key.
fn interior_width() -> usize {
    let widest = SortRow::ALL
        .iter()
        .map(|row| Cells::of(row.label()).get() + GAP + Cells::of(&row.shortcut()).get())
        .max()
        .unwrap_or(0);
    PAD + 2 + widest + PAD
}

fn render_row(f: &mut Frame, row: SortRow, menu: &SortMenu, ground: Style, area: Rect) {
    let selected = menu.row() == row;
    let style = if selected {
        ground
            .bg(palette::MENU_SELECTED_BG)
            .fg(palette::MENU_SELECTED_FG)
    } else {
        ground
    };
    let tick = if menu.is_ticked(row) { TICK } else { ' ' };
    let shortcut = row.shortcut();

    let mut spans = vec![Span::styled(format!("{}{tick} ", " ".repeat(PAD)), style)];
    spans.extend(hotkey_spans(row.label(), style, !selected));
    let used = PAD + 2 + Cells::of(row.label()).get();
    let gap = usize::from(area.width)
        .saturating_sub(used)
        .saturating_sub(Cells::of(&shortcut).get())
        .saturating_sub(PAD);
    spans.push(Span::styled(" ".repeat(gap), style));
    spans.push(Span::styled(shortcut, style));
    spans.push(Span::styled(" ".repeat(PAD), style));

    f.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
}

/// The label with its first letter — the row's hotkey — picked out, except on
/// the selected row, where the inversion already says everything.
fn hotkey_spans<'a>(label: &str, style: Style, accented: bool) -> Vec<Span<'a>> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return Vec::new();
    };
    let accent = if accented {
        style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD)
    } else {
        style
    };
    vec![
        Span::styled(first.to_string(), accent),
        Span::styled(chars.as_str().to_owned(), style),
    ]
}

/// The rule between the modes and the order switch, joined to the frame at
/// both ends so it reads as part of it.
fn render_rule(f: &mut Frame, ground: Style, area: Rect) {
    let rule = format!("╟{}╢", "─".repeat(usize::from(area.width)));
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(rule, ground))).style(ground),
        Rect {
            x: area.x.saturating_sub(1),
            width: area.width + 2,
            ..area
        },
    );
}

#[cfg(test)]
mod tests;
