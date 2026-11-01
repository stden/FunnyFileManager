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
use super::styles::{cursor_style, entry_colour, entry_style, mark_of, separator_style, Mark};
use crate::models::{DirEntry, Panel, ShortNames};
use crate::scroll::visible_window;

/// A stripe with no entry left to show: nothing in it but the column rules.
///
/// What makes the panel read as a table below the last file: Far's rules run
/// to the divider, and a rule that stops where the data stops leaves the frame
/// looking broken in every directory with fewer files than rows.
fn empty_cell(name: Cells, columns: &[PanelColumn]) -> Span<'static> {
    let mut cell = " ".repeat(name.get());
    for column in columns {
        cell.push_str(COLUMN_SEPARATOR);
        cell.push_str(&" ".repeat(column.width().get()));
    }
    Span::styled(cell, separator_style())
}

/// One entry in one stripe: the name, then each column behind its rule.
fn entry_cell(
    panel: &Panel,
    short_names: Option<&ShortNames>,
    entry: &DirEntry,
    name: Cells,
    columns: &[PanelColumn],
) -> Vec<Span<'static>> {
    let mark = mark_of(panel, entry);
    let hidden = panel.is_hidden(&entry.name);
    let displayed = displayed_entry(short_names, entry);
    let name_style = entry_style(panel, entry, mark);
    // Marked, hidden and executable rows paint their whole row — name,
    // extension, size, date and time — in the entry's colour, as Far's file
    // highlighting does while keeping the `│` rules cyan.
    let whole_row = mark == Mark::Marked || hidden || (entry.executable && !entry.kind.is_dir());
    let data_style = Style::default().fg(if whole_row {
        entry_colour(panel, entry, mark)
    } else {
        palette::DATA
    });

    let mut spans = vec![Span::styled(name_field(&displayed, name), name_style)];
    for column in columns {
        spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        spans.push(Span::styled(
            column.lay_out(&column.cell(&displayed)),
            data_style,
        ));
    }
    spans
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
    let lines: Vec<Line<'_>> = (0..rows)
        .map(|row| {
            let cells = (0..layout.stripes).map(|stripe| {
                Some(window.start + stripe * rows + row).filter(|index| window.contains(index))
            });
            body_line(panel, short_names.as_ref(), layout, cells, cursor)
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(FAR_BLUE)),
        body,
    );
    render_empty_hint(f, panel, body);
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

/// One body row: the entry each stripe shows on it, or — where the listing
/// has run out — just the column rules, which run the full height of the body.
fn body_line(
    panel: &Panel,
    short_names: Option<&ShortNames>,
    layout: PanelLayout,
    cells: impl Iterator<Item = Option<usize>>,
    cursor: Option<usize>,
) -> Line<'static> {
    let mut spans = Vec::new();
    for (stripe, index) in cells.enumerate() {
        if stripe > 0 {
            spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        }
        let name = layout.name_width(stripe);
        let entry = index.and_then(|index| panel.entries.get(index));
        let (Some(index), Some(entry)) = (index, entry) else {
            spans.push(empty_cell(name, layout.columns));
            continue;
        };
        let cell = entry_cell(panel, short_names, entry, name, layout.columns);
        // The cursor covers its own stripe only, never the rule beside it.
        if cursor == Some(index) {
            let style = cursor_style(mark_of(panel, entry));
            spans.extend(cell.into_iter().map(|span| span.patch_style(style)));
        } else {
            spans.extend(cell);
        }
    }
    Line::from(spans)
}

/// The entry as the row shows it: itself, or a copy under its 8.3 name while
/// the panel shows short names, with anything the terminal will not draw
/// replaced. Only the name changes — marks, hiding and every operation still
/// go by the real one.
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
