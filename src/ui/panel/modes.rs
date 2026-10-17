//! Render the one-column detail views — Detailed, the descriptions, owners,
//! links and Alternative full — from cached directory facts. Brief, Medium,
//! Full and Wide are tables, drawn by the parent module.

use ratatui::{layout::Rect, text::Line, widgets::Paragraph, Frame};

use super::{Cells, Style};
use crate::models::{DirEntry, Panel, ShortName, ShortNames};
use crate::scroll::visible_window;

mod columns;

pub(super) fn render(f: &mut Frame<'_>, panel: &mut Panel, inner: Rect, active: bool) {
    let header = Rect { height: 1, ..inner };
    let layout = columns::Layout::new(panel.view, Cells::columns(inner.width));
    f.render_widget(Paragraph::new(layout.header()), header);
    let body = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(3),
        ..inner
    };
    if body.height == 0 {
        return;
    }
    render_rows(f, panel, body, active, &layout);
}

fn render_rows(
    f: &mut Frame<'_>,
    panel: &mut Panel,
    body: Rect,
    active: bool,
    layout: &columns::Layout,
) {
    let offset = panel.list_state_mut().offset();
    let window = visible_window(
        panel.entries.len(),
        usize::from(body.height),
        offset,
        panel.selected(),
    );
    *panel.list_state_mut().offset_mut() = window.start;
    let short_names = short_names(panel);
    for row in 0..body.height {
        let area = Rect {
            y: body.y + row,
            height: 1,
            ..body
        };
        let index = window.start + usize::from(row);
        let Some(entry) = panel.entries.get(index) else {
            let rule = super::separator_style().bg(super::FAR_BLUE);
            f.render_widget(Paragraph::new(layout.blank()).style(rule), area);
            continue;
        };
        let style = row_style(panel, entry, active && panel.selected() == Some(index));
        let name = displayed_name(short_names.as_ref(), &entry.name);
        f.render_widget(
            Paragraph::new(Line::from(layout.row(panel, entry, &name))).style(style),
            area,
        );
    }
}

/// The 8.3 forms of the listing while the panel shows short names; `None`
/// while it shows long ones. Built once per draw, not per row: a name's `~N`
/// depends on every name before it, so the table is one pass over the whole
/// listing.
pub(super) fn short_names(panel: &Panel) -> Option<ShortNames> {
    panel
        .options
        .short_names
        .then(|| ShortNames::for_entries(&panel.entries))
}

/// What a row shows for `name`: its 8.3 form where the table has one, else
/// the name as it is — in both cases with anything undrawable replaced.
pub(super) fn displayed_name<'a>(
    table: Option<&'a ShortNames>,
    name: &'a str,
) -> std::borrow::Cow<'a, str> {
    crate::ui::printable_name(
        table
            .and_then(|table| table.get(name))
            .map_or(name, ShortName::as_str),
    )
}

fn row_style(panel: &Panel, entry: &DirEntry, selected: bool) -> Style {
    let mark = super::mark_of(panel, entry);
    if selected {
        return super::cursor_style(mark);
    }
    super::entry_style(panel, entry, mark).bg(super::FAR_BLUE)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_details;
