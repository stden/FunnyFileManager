//! Translate panel columns and row clicks using the same view geometry as rendering.

use std::io::Write;
use std::time::Instant;

use ratatui::{backend::Backend, layout::Rect, Terminal};

use crate::{
    action::Action,
    models::{App, Panel, PanelKind, PanelSide},
    mouse::{PanelClicks, PanelRowClick, Target},
};

/// How many columns `panel` actually draws on screen: only a `Files` panel can
/// use a multi-column `Brief` or `Medium` layout; `Tree`, `QuickView`,
/// `Sessions` and `Remote` are always single-column.
pub(super) const fn columns(panel: &Panel) -> usize {
    if matches!(panel.kind, PanelKind::Files) {
        panel.view.columns()
    } else {
        1
    }
}

/// Entries one page holds in the active panel's view: the visible rows, times
/// the columns a multi-column view lays them out in. Every view spends the
/// same three non-entry rows inside the frame, so the row count is shared.
pub(super) fn page(app: &App, rows: usize) -> usize {
    rows.max(1) * columns(app.get_active_panel_ref())
}

/// Hit-test a panel screen coordinate, accounting for the `Tree` panel's
/// headerless body and multi-column `Files` stripes.
pub(super) fn hit(app: &App, area: Rect, column: u16, row: u16) -> Target {
    let raw = crate::mouse::hit(
        area,
        column,
        row,
        crate::ui::FKEY_COUNT,
        app.panels_on_screen(),
    );
    if let Target::Panel(side) | Target::PanelRow { side, .. } = raw {
        if app.panel(side).kind.is_tree() {
            return tree_target(area, row, side);
        }
    }
    target(app, raw, area, column)
}

fn tree_target(area: Rect, row: u16, side: PanelSide) -> Target {
    let panels_height = area.height.saturating_sub(3);
    let panel_row = row.saturating_sub(area.y + 1);
    if panels_height > 3 && panel_row >= 1 && panel_row + 2 < panels_height {
        Target::PanelRow {
            side,
            row: usize::from(panel_row - 1),
        }
    } else {
        Target::Panel(side)
    }
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
    if panel.kind.is_quick_view() {
        return Target::Panel(side);
    }
    if columns(panel) == 1 {
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

pub(super) fn click_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: PanelSide,
    row: usize,
    panel_clicks: &mut PanelClicks,
) {
    app.active_panel = side;
    match &app.get_active_panel_ref().kind {
        PanelKind::QuickView(_) => panel_clicks.reset(),
        PanelKind::Sessions(_) | PanelKind::Remote(_) => {
            click_netbox_row(terminal, app, side, row, panel_clicks);
        }
        PanelKind::Tree(_) => click_tree_row(terminal, app, side, row, panel_clicks),
        PanelKind::Files => click_file_row(terminal, app, side, row, panel_clicks),
    }
}

fn click_file_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: PanelSide,
    row: usize,
    panel_clicks: &mut PanelClicks,
) {
    let panel = app.get_active_panel();
    let index = panel.list_state_mut().offset() + row;
    if index >= panel.entries.len() {
        panel_clicks.reset();
        return;
    }
    let click = panel_clicks.press(side, index, Instant::now());
    panel.select(Some(index));
    if click == PanelRowClick::Activate {
        super::dispatch_action(terminal, app, Action::Open);
    }
}

fn click_tree_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: PanelSide,
    row: usize,
    panel_clicks: &mut PanelClicks,
) {
    let index = {
        let PanelKind::Tree(tree) = &mut app.get_active_panel().kind else {
            return;
        };
        let index = tree.scroll.saturating_add(row);
        if index >= tree.rows().len() {
            panel_clicks.reset();
            return;
        }
        tree.cursor = index;
        index
    };
    crate::ops::tree::follow_cursor(app);
    if panel_clicks.press(side, index, Instant::now()) == PanelRowClick::Activate {
        super::dispatch_action(terminal, app, Action::Open);
    }
}

/// A click on a row of a `NetBox` panel: the panel's own cursor moves there —
/// never the hidden file list's — and a double click opens it, as Enter does.
fn click_netbox_row<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    side: PanelSide,
    row: usize,
    panel_clicks: &mut PanelClicks,
) {
    let Some(index) = app.get_active_panel().kind.select_visible_row(row) else {
        panel_clicks.reset();
        return;
    };
    if panel_clicks.press(side, index, Instant::now()) == PanelRowClick::Activate {
        super::dispatch_action(terminal, app, Action::Open);
    }
}
