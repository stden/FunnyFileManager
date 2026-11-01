//! Purpose: Draws a completion dropdown: the command line's, above the prompt, and the Language servers form's, below its field — each flipping when the other side has more room.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem},
    Frame,
};

use crate::models::{App, Overlay};

use super::layout::{MENU_MAX_ROWS, PANEL_BORDERS};
use super::{far_block, palette, Cells, FAR_BLUE};

/// Where a dropdown hangs from its anchor row, by preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Above,
    Below,
}

/// Whether the dropdown shows its selected row. A passive list, opened by
/// typing in a form, highlights nothing: Enter still belongs to the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Highlight {
    Selected,
    None,
}

/// Draw the completion dropdown directly above the command line.
///
/// The window scrolls to keep the highlighted candidate visible, so a long
/// candidate list stays navigable without growing the popup.
pub(super) fn render_completion_menu(f: &mut Frame<'_>, app: &App, command_line_area: Rect) {
    let Overlay::CommandLine(state) = &app.overlay else {
        return;
    };
    let Some(menu) = &state.menu else {
        return;
    };
    // The command line is not a tracked dialog: nothing records the area.
    render_dropdown(f, menu, command_line_area, Side::Above, Highlight::Selected);
}

/// Draw `menu` hanging from the `anchor` row on its preferred `side`, or on
/// the other side when the screen has more room there. Returns where it was
/// drawn, which a dialog records so a click on it reaches nothing beneath.
pub(super) fn render_dropdown(
    f: &mut Frame<'_>,
    menu: &crate::models::Menu,
    anchor: Rect,
    side: Side,
    highlight: Highlight,
) -> Option<Rect> {
    // No room for even one candidate row — an empty bordered box would just
    // be noise.
    let (area, visible) = completion_placement(f.area(), anchor, menu.candidates().len(), side)?;
    // Scroll so `selected` sits inside the window.
    let first = menu.selected().saturating_sub(visible - 1);

    let items = completion_items(menu, area, first, visible, highlight);
    let list = List::new(items).block(
        far_block()
            .border_style(Style::default().fg(palette::COMPLETION_BG))
            .style(Style::default().bg(FAR_BLUE)),
    );

    f.render_widget(Clear, area);
    f.render_widget(list, area);
    Some(area)
}

fn completion_items(
    menu: &crate::models::Menu,
    area: Rect,
    first: usize,
    visible: usize,
    highlight: Highlight,
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
            let lit = highlight == Highlight::Selected && index == menu.selected();
            let style = if lit {
                Style::default()
                    .bg(palette::COMPLETION_BG)
                    .fg(palette::HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::COMPLETION_BG)
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
/// On the preferred `side` of the anchor row when it has at least as much room
/// as the other — the command line's list hangs above its prompt, as Far's
/// does, and a form's hangs below its field. The console's prompt follows its
/// output, so on a fresh console it sits at the very top with nothing above
/// it; there the popup drops below instead, because a completion list that
/// silently fails to appear is worse than one on the unexpected side.
///
/// The row budget comes from the space actually available, not from the
/// wish-list [`MENU_MAX_ROWS`]: sizing the scroll window larger than the area
/// makes `render_widget` (which draws with a fresh, unselected `ListState`)
/// truncate from the end of the item slice to fit, and that truncation can cut
/// off exactly the row carrying `selected`.
fn completion_placement(
    screen: Rect,
    row: Rect,
    candidates: usize,
    side: Side,
) -> Option<(Rect, usize)> {
    let wanted = candidates.min(MENU_MAX_ROWS);
    // A popup needs its two border rows before it can show a candidate at all.
    let rows = |span: u16| usize::from(span.saturating_sub(PANEL_BORDERS));
    let above = wanted.min(rows(row.y.saturating_sub(screen.y)));
    let below = wanted.min(rows(
        (screen.y + screen.height).saturating_sub(row.y + row.height),
    ));
    let goes_above = match side {
        Side::Above => above >= below,
        Side::Below => above > below,
    };
    let (visible, y) = if goes_above {
        // Exact rather than saturating by construction: `above` already set
        // aside the borders, so the box cannot reach past the top of the screen.
        (above, row.y.saturating_sub(popup_height(above)))
    } else {
        (below, row.y + row.height)
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

#[cfg(test)]
mod tests;
