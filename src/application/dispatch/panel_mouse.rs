//! Translate panel columns using the same view geometry as rendering.

use crate::{
    models::{App, PanelSide},
    mouse::Target,
};
use ratatui::layout::Rect;

/// Entries one page holds in the active panel's view: the visible rows, times
/// the columns a multi-column view lays them out in. Every view spends the
/// same three non-entry rows inside the frame, so the row count is shared.
pub(super) fn page(app: &App, rows: usize) -> usize {
    rows.max(1) * app.get_active_panel_ref().view.columns()
}

/// `target` comes from `mouse::hit`, which already routes every column to a
/// panel that is drawn; this only spreads the row across the columns of a
/// multi-column view, over the width `ui::panel_areas` gave that panel — the
/// whole screen when it is alone on it.
pub(super) fn target(app: &App, target: Target, area: Rect, column: u16) -> Target {
    let Target::PanelRow { side, row } = target else {
        return target;
    };
    let panel = app.panel(side);
    if panel.view.columns() == 1 {
        return target;
    }
    // The renderer's own split says where the panel was drawn — only its
    // columns matter here — so an odd width or a lone panel cannot put a
    // click one stripe off.
    let (left, right) = crate::ui::panel_areas(app, area);
    let Some(drawn) = (match side {
        PanelSide::Left => left,
        PanelSide::Right => right,
    }) else {
        return Target::Nothing;
    };
    let interior = drawn.width.saturating_sub(2);
    let Some(x) = column.checked_sub(drawn.x + 1).filter(|x| *x < interior) else {
        return Target::Nothing;
    };
    let stripe = crate::ui::stripe_at(
        panel.view,
        crate::ui::Cells::columns(interior),
        usize::from(x),
    );
    Target::PanelRow {
        side,
        row: row + stripe * crate::ui::panel_page_size(area.height),
    }
}
