//! Purpose: Draws one file panel — its columns, header, rows and footer.
//!
//! Split out of `ui.rs` when the Far-style columns pushed that file past the
//! 1000-line cap `tests/file_line_limit_test.rs` enforces. The boundary is not
//! arbitrary: everything here is about the inside of a panel, and `ui.rs` keeps
//! the screen it sits on.
//!
//! Each part lives in a child named for it — `columns`, `layout`, `fields`,
//! `styles`, `table`, `rows`, `frame`, `info` — and this file draws the panel
//! from them and re-exports what the rest of `ui` uses.

mod columns;
mod fields;
mod frame;
mod info;
mod layout;
mod modes;
mod rows;
mod selection;
mod styles;
mod table;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use super::palette;
use super::{head_cells, Cells, FAR_BLUE, PANEL_BORDERS};
use super::{PANEL_COLUMN_HEADER, PANEL_COLUMN_RULE, PANEL_INFO_ROW, PANEL_NON_ENTRY_ROWS};
use crate::models::Panel;
use crate::status::PathMessage;
pub(super) use columns::COLUMN_SEPARATOR;
use columns::{info_columns, table_columns};
pub(super) use fields::group_digits;
pub(super) use fields::{
    abbreviate_bytes, center_cells, clip_name, pad_cells, right_cells, size_field,
};
#[cfg(test)]
pub(super) use fields::{name_field, split_extension};
use frame::panel_block;
pub(super) use info::panel_footer;
use info::render_panel_info;
pub(super) use layout::panel_layout;
pub use layout::stripe_at;
use rows::render_entries;
use styles::mark_of;
pub(super) use styles::{cursor_colour, cursor_style, separator_style, Mark};
use table::{header_line, render_column_rule};

pub(super) fn render_panel(
    f: &mut Frame<'_>,
    panel: &mut Panel,
    area: Rect,
    is_active: bool,
    footer: Option<&PathMessage<'_>>,
) {
    let border_style = if is_active {
        Style::default()
            .fg(palette::FRAME)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette::FRAME)
    };
    let interior = Cells::columns(area.width.saturating_sub(PANEL_BORDERS));
    let block = panel_block(panel, interior, is_active, border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    render_panel_contents(f, panel, area, inner, is_active, footer, border_style);
}

fn render_panel_contents(
    f: &mut Frame<'_>,
    panel: &mut Panel,
    area: Rect,
    inner: Rect,
    is_active: bool,
    footer: Option<&PathMessage<'_>>,
    border_style: Style,
) {
    let Some(shown) = table_columns(panel.view) else {
        modes::render(f, panel, inner, is_active);
        if inner.height >= PANEL_NON_ENTRY_ROWS {
            selection::render(f, panel, divider_row(inner));
        }
        render_panel_info(f, panel, inner, footer, &[]);
        return;
    };
    let layout = panel_layout(panel.view, Cells::columns(inner.width));
    f.render_widget(
        Paragraph::new(header_line(panel.sort_mode, layout)).style(Style::default().bg(FAR_BLUE)),
        Rect { height: 1, ..inner },
    );
    if inner.height < PANEL_NON_ENTRY_ROWS {
        return;
    }

    let rows = inner.height - PANEL_NON_ENTRY_ROWS;

    let body = Rect {
        y: inner.y + PANEL_COLUMN_HEADER,
        height: rows,
        ..inner
    };
    render_entries(f, panel, body, layout, is_active);

    let divider = divider_row(inner);
    render_column_rule(f, layout, area, divider, border_style);
    selection::render(f, panel, divider);

    render_panel_info(f, panel, inner, footer, &info_columns(shown));
}

/// The row between the entries and the info row: the column rule in the
/// table views, blank in the detail views, and in both the place Far writes
/// its selection summary.
fn divider_row(inner: Rect) -> Rect {
    Rect {
        y: inner.y
            + inner
                .height
                .saturating_sub(PANEL_COLUMN_RULE + PANEL_INFO_ROW),
        height: PANEL_COLUMN_RULE,
        ..inner
    }
}

#[cfg(test)]
mod tests_cursor;
#[cfg(test)]
mod tests_header;
#[cfg(test)]
mod tests_link_sizes;
#[cfg(test)]
mod tests_selection;
#[cfg(test)]
mod tests_sizes;
#[cfg(test)]
mod tests_snapshot;
#[cfg(test)]
mod tests_title;
#[cfg(test)]
mod tests_views;
