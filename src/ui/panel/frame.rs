//! Purpose: A panel's double frame: the path in its title, highlighted on
//! the active panel, and the totals and free space in its bottom border.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::super::{far_block, tail_cells, Cells, FAR_BLUE};
use super::fields::abbreviate_bytes;
use crate::models::Panel;

/// The totals Far prints into the panel's bottom border.
///
/// Counts files rather than rows: the directories and the ".." link are not
/// what "381 K" is the size of, so including them in the denominator would
/// make the two halves of `(10/30)` measure different things.
fn panel_totals(panel: &Panel) -> String {
    let (count, bytes) = panel.file_totals();
    format!(
        " {} ({count}/{}) ",
        abbreviate_bytes(bytes),
        panel.entries.len()
    )
}

pub(super) fn panel_block(
    panel: &Panel,
    interior: Cells,
    is_active: bool,
    border_style: Style,
) -> ratatui::widgets::Block<'static> {
    let mut block = far_block()
        .border_style(border_style)
        .style(Style::default().bg(FAR_BLUE))
        .title(panel_title(panel, interior, is_active))
        .title_bottom(
            Line::from(Span::styled(
                panel_totals(panel),
                Style::default().fg(Color::White).bg(FAR_BLUE),
            ))
            .centered(),
        );
    if let Some(free) = panel.available_space() {
        block = block.title_bottom(
            Line::from(Span::styled(
                format!(" {} ", abbreviate_bytes(free.bytes())),
                Style::default().fg(Color::White).bg(FAR_BLUE),
            ))
            .right_aligned(),
        );
    }

    block
}

fn panel_title(panel: &Panel, interior: Cells, is_active: bool) -> Line<'static> {
    // Far highlights the active panel's path; an unhighlighted title is how
    // the inactive panel says so even when neither has a visible cursor bar.
    let title_style = super::super::active_title_style(is_active);

    // Far's `*` before the path: the listing is not the whole directory.
    let filtered = if panel.filter.is_active() { "*" } else { "" };
    // The blanks around the path and the `*` share the frame's width with it.
    // A title wider than the frame is cut by ratatui from its left end, which
    // took the blank, the ellipsis and the first kept character with it and
    // left a cut path reading as a relative one.
    let room = interior.saturating_sub(Cells(2) + Cells::of(filtered));
    let raw = crate::path_display::label(&panel.path);
    let label = crate::ui::printable_name(&raw);
    let path = tail_cells(&label, room);
    Line::from(Span::styled(format!(" {filtered}{path} "), title_style)).centered()
}
