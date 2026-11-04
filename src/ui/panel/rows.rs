//! Purpose: A panel's body rows: only the entries in the visible window, each
//! stripe's name and columns in the row's colours, the cursor bar over its
//! own stripe, and bare column rules below the last entry.

use std::borrow::Cow;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{palette, Cells, FAR_BLUE};
use super::columns::{PanelColumn, COLUMN_SEPARATOR};
use super::fields::name_field;
use super::layout::PanelLayout;
use super::modes;
use super::styles::{cursor_style, row_facts, separator_style, Mark};
use crate::models::{DirEntry, Panel, ShortNames};
use crate::scroll::visible_window;

/// A stripe with no entry left to show: nothing in it but the column rules.
fn empty_cell_text(name: Cells, columns: &[PanelColumn]) -> String {
    let extra: usize = columns
        .iter()
        .map(|col| COLUMN_SEPARATOR.len() + col.width().get())
        .sum();
    let mut cell = String::with_capacity(name.get() + extra);
    super::fields::push_spaces(&mut cell, name.get());
    for column in columns {
        cell.push_str(COLUMN_SEPARATOR);
        super::fields::push_spaces(&mut cell, column.width().get());
    }
    cell
}

/// One entry in one stripe: the name, then each column behind its rule.
fn entry_cell(
    panel: &Panel,
    short_names: Option<&ShortNames>,
    entry: &DirEntry,
    name: Cells,
    columns: &[PanelColumn],
) -> (Mark, Vec<Span<'static>>) {
    let facts = row_facts(panel, entry);
    let displayed = displayed_entry(short_names, entry);
    let mut spans = Vec::with_capacity(1 + columns.len() * 2);
    spans.push(Span::styled(name_field(&displayed, name), facts.name_style));
    let sep_style = separator_style();
    for column in columns {
        spans.push(Span::styled(COLUMN_SEPARATOR, sep_style));
        spans.push(Span::styled(
            column.lay_out(column.cell(&displayed)),
            facts.data_style,
        ));
    }
    (facts.mark, spans)
}

pub(super) fn render_entries(
    f: &mut Frame<'_>,
    panel: &mut Panel,
    body: Rect,
    layout: PanelLayout,
    is_active: bool,
) {
    // Only the rows that will be drawn are built.
    let rows = usize::from(body.height);
    let window = visible_window(
        panel.entries.len(),
        rows * layout.stripes,
        panel.list_state_mut().offset(),
        panel.selected(),
    );
    *panel.list_state_mut().offset_mut() = window.start;

    // Far draws no cursor on the inactive panel: two highlighted rows would
    // give the screen two answers to "what does Enter act on".
    let cursor = panel.selected().filter(|_| is_active);
    let panel = &*panel;
    let short_names = modes::short_names(panel);
    let empty_cells: Vec<String> = (0..layout.stripes)
        .map(|stripe| empty_cell_text(layout.name_width(stripe), layout.columns))
        .collect();
    let lines = build_body_lines(
        panel,
        short_names.as_ref(),
        layout,
        rows,
        &window,
        &empty_cells,
        cursor,
    );
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(FAR_BLUE)),
        body,
    );
    render_empty_hint(f, panel, body);
}

fn build_body_lines<'a>(
    panel: &'a Panel,
    short_names: Option<&'a ShortNames>,
    layout: PanelLayout,
    rows: usize,
    window: &std::ops::Range<usize>,
    empty_cells: &'a [String],
    cursor: Option<usize>,
) -> Vec<Line<'a>> {
    (0..rows)
        .map(|row| {
            let cells = (0..layout.stripes).map(|stripe| {
                Some(window.start + stripe * rows + row).filter(|index| window.contains(index))
            });
            body_line(panel, short_names, layout, empty_cells, cells, cursor)
        })
        .collect()
}

/// Give an empty listing a useful action without covering its parent row.
fn render_empty_hint(f: &mut Frame<'_>, panel: &Panel, body: Rect) {
    if panel.entries.iter().any(|entry| entry.name != "..") {
        return;
    }
    let first_free = u16::from(!panel.entries.is_empty());
    if body.height <= first_free {
        return;
    }
    let hint = if panel.is_filtered() {
        "No matches · F9 > Commands"
    } else {
        "No visible entries · F7: new folder"
    };
    let area = Rect {
        y: body.y + (body.height / 2).max(first_free),
        height: 1,
        ..body
    };
    f.render_widget(
        Paragraph::new(Line::from(hint).centered())
            .style(Style::default().fg(palette::DATA).bg(FAR_BLUE)),
        area,
    );
}

/// One body row: the entry each stripe shows on it, or bare column rules below the last entry.
fn body_line<'a>(
    panel: &Panel,
    short_names: Option<&ShortNames>,
    layout: PanelLayout,
    empty_cells: &'a [String],
    cells: impl Iterator<Item = Option<usize>>,
    cursor: Option<usize>,
) -> Line<'a> {
    let mut spans = Vec::with_capacity(layout.stripes * (2 + layout.columns.len() * 2));
    let sep_style = separator_style();
    for (stripe, index) in cells.enumerate() {
        if stripe > 0 {
            spans.push(Span::styled(COLUMN_SEPARATOR, sep_style));
        }
        let name = layout.name_width(stripe);
        let entry = index.and_then(|index| panel.entries.get(index));
        let (Some(index), Some(entry)) = (index, entry) else {
            if let Some(blank) = empty_cells.get(stripe) {
                spans.push(Span::styled(blank.as_str(), sep_style));
            }
            continue;
        };
        let (mark, cell) = entry_cell(panel, short_names, entry, name, layout.columns);
        if cursor == Some(index) {
            let style = cursor_style(mark);
            spans.extend(cell.into_iter().map(|span| span.patch_style(style)));
        } else {
            spans.extend(cell);
        }
    }
    Line::from(spans)
}

/// The entry as the row shows it.
fn displayed_entry<'a>(short_names: Option<&ShortNames>, entry: &'a DirEntry) -> Cow<'a, DirEntry> {
    let short = short_names.and_then(|table| table.get(&entry.name));
    let shown = short.map_or(entry.name.as_str(), crate::models::ShortName::as_str);
    let printable = crate::ui::printable_name(shown);
    if short.is_none() && matches!(printable, Cow::Borrowed(_)) {
        return Cow::Borrowed(entry);
    }
    let mut displayed = entry.clone();
    displayed.name = printable.into_owned();
    Cow::Owned(displayed)
}
