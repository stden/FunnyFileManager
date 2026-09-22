//! Purpose: Draws the command-line completion dropdown, above the prompt or below it when there is no room.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem},
    Frame,
};

use crate::models::{App, Overlay};

use super::layout::{MENU_MAX_ROWS, PANEL_BORDERS};
use super::{far_block, Cells, FAR_BLUE};

/// Draw the completion dropdown directly above the command line.
///
/// The window scrolls to keep the highlighted candidate visible, so a long
/// candidate list stays navigable without growing the popup.
pub(super) fn render_completion_menu(f: &mut Frame, app: &App, command_line_area: Rect) {
    let Overlay::CommandLine(state) = &app.overlay else {
        return;
    };
    let Some(menu) = &state.menu else {
        return;
    };

    // No room for even one candidate row — an empty bordered box would just
    // be noise.
    let Some((area, visible)) =
        completion_placement(f.area(), command_line_area, menu.candidates().len())
    else {
        return;
    };
    // Scroll so `selected` sits inside the window.
    let first = menu.selected().saturating_sub(visible - 1);

    let items = completion_items(menu, area, first, visible);
    let list = List::new(items).block(
        far_block()
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(FAR_BLUE)),
    );

    f.render_widget(Clear, area);
    f.render_widget(list, area);
}

fn completion_items(
    menu: &crate::models::Menu,
    area: Rect,
    first: usize,
    visible: usize,
) -> Vec<ListItem<'static>> {
    let inner_width = Cells::columns(area.width.saturating_sub(PANEL_BORDERS));
    menu.candidates()
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .map(|(index, candidate)| {
            let hint = candidate.hint.as_deref().unwrap_or("");
            // Cells, not chars — same rule as `fit_name`: a char count
            // over-fills the row for double-width glyphs and pushes the
            // right-aligned hint off the popup.
            let gap = inner_width.saturating_sub(Cells::of(&candidate.text) + Cells::of(hint));
            let style = if index == menu.selected() {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}{hint}", candidate.text, " ".repeat(gap.get())),
                style,
            )))
        })
        .collect()
}

/// Where the completion popup goes, and how many candidate rows it shows.
///
/// Above the prompt by preference — that is where Far puts it, and in the
/// panel view there is always room. The console's prompt follows its output, so
/// on a fresh console it sits at the very top with nothing above it; there the
/// popup drops below instead, because a completion list that silently fails to
/// appear is worse than one on the unexpected side.
///
/// The row budget comes from the space actually available, not from the
/// wish-list [`MENU_MAX_ROWS`]: sizing the scroll window larger than the area
/// makes `render_widget` (which draws with a fresh, unselected `ListState`)
/// truncate from the end of the item slice to fit, and that truncation can cut
/// off exactly the row carrying `selected`.
fn completion_placement(screen: Rect, row: Rect, candidates: usize) -> Option<(Rect, usize)> {
    let wanted = candidates.min(MENU_MAX_ROWS);
    // A popup needs its two border rows before it can show a candidate at all.
    let rows = |span: u16| usize::from(span.saturating_sub(PANEL_BORDERS));
    let above = rows(row.y.saturating_sub(screen.y));
    let below = rows((screen.y + screen.height).saturating_sub(row.y + row.height));

    let (visible, y) = if wanted.min(above) >= wanted.min(below) {
        let visible = wanted.min(above);
        // Exact rather than saturating by construction: `above` already set
        // aside the borders, so the box cannot reach past the top of the screen.
        (visible, row.y.saturating_sub(popup_height(visible)))
    } else {
        (wanted.min(below), row.y + row.height)
    };
    if visible == 0 {
        return None;
    }
    Some((
        Rect {
            x: row.x,
            y,
            width: row.width,
            height: popup_height(visible),
        },
        visible,
    ))
}

/// Rows a popup showing `visible` candidates occupies, its borders included.
fn popup_height(visible: usize) -> u16 {
    u16::try_from(visible)
        .unwrap_or(u16::MAX)
        .saturating_add(PANEL_BORDERS)
}
