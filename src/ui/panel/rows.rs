//! Purpose: Panel body rows: visible-window stripes, cursor highlight, and
//! trailing column rules written directly into the terminal buffer.

use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line, widgets::Paragraph, Frame};
use std::borrow::Cow;

use super::super::{is_printable_ascii, palette, screen_u16, Cells, FAR_BLUE};
use super::columns::{PanelColumn, COLUMN_SEPARATOR};
use super::fields::name_field;
use super::layout::PanelLayout;
use super::modes;
use super::styles::{cursor_style, row_facts, separator_style};
use crate::models::{DirEntry, Panel, ShortNames};
use crate::scroll::visible_window;

pub(super) fn render_entries(
    f: &mut Frame<'_>,
    panel: &mut Panel,
    body: Rect,
    layout: PanelLayout,
    is_active: bool,
) {
    let rows = usize::from(body.height);
    let window = visible_window(
        panel.entries.len(),
        rows * layout.stripes,
        panel.list_state_mut().offset(),
        panel.selected(),
    );
    *panel.list_state_mut().offset_mut() = window.start;
    // Far draws no cursor on the inactive panel so Enter has one clear target.
    let cursor = panel.selected().filter(|_| is_active);
    let panel = &*panel;
    let short_names = modes::short_names(panel);
    let buf = f.buffer_mut();
    let clipped = body.intersection(buf.area);
    buf.set_style(clipped, Style::default().bg(FAR_BLUE));
    for row in 0..usize::from(clipped.height) {
        let y = clipped.y + screen_u16(row);
        render_row(
            buf,
            (clipped.x, y, clipped.right()),
            panel,
            short_names.as_ref(),
            layout,
            (rows, row, &window),
            cursor,
        );
    }
    render_empty_hint(f, panel, body);
}

fn render_row(
    buf: &mut Buffer,
    (mut x, y, max_x): (u16, u16, u16),
    panel: &Panel,
    short_names: Option<&ShortNames>,
    layout: PanelLayout,
    (rows, row, window): (usize, usize, &std::ops::Range<usize>),
    cursor: Option<usize>,
) {
    let sep_style = separator_style();
    for stripe in 0..layout.stripes {
        if stripe > 0 {
            x = write_row_text(buf, x, y, max_x, COLUMN_SEPARATOR, sep_style);
        }
        let name = layout.name_width(stripe);
        let index = Some(window.start + stripe * rows + row).filter(|idx| window.contains(idx));
        let entry = index.and_then(|idx| panel.entries.get(idx));
        if let (Some(idx), Some(entry)) = (index, entry) {
            x = render_entry_stripe(
                buf,
                (x, y, max_x),
                panel,
                short_names,
                entry,
                (name, layout.columns),
                cursor == Some(idx),
            );
        } else {
            x = render_empty_stripe(buf, (x, y, max_x), (name, layout.columns), sep_style);
        }
    }
}

fn render_empty_stripe(
    buf: &mut Buffer,
    (mut x, y, max_x): (u16, u16, u16),
    (name, columns): (Cells, &[PanelColumn]),
    sep_style: Style,
) -> u16 {
    x = write_spaces(buf, x, y, max_x, name.get(), sep_style);
    for column in columns {
        x = write_row_text(buf, x, y, max_x, COLUMN_SEPARATOR, sep_style);
        x = write_spaces(buf, x, y, max_x, column.width().get(), sep_style);
    }
    x
}

fn render_entry_stripe(
    buf: &mut Buffer,
    (mut x, y, max_x): (u16, u16, u16),
    panel: &Panel,
    short_names: Option<&ShortNames>,
    entry: &DirEntry,
    (name, columns): (Cells, &[PanelColumn]),
    is_cursor: bool,
) -> u16 {
    let facts = row_facts(panel, entry);
    let displayed = displayed_entry(short_names, entry);
    let patch = is_cursor.then(|| cursor_style(facts.mark));
    let name_style = patch.map_or(facts.name_style, |bar| facts.name_style.patch(bar));
    let sep_base = separator_style();
    let sep_style = patch.map_or(sep_base, |bar| sep_base.patch(bar));
    let data_style = patch.map_or(facts.data_style, |bar| facts.data_style.patch(bar));
    let formatted_name = name_field(&displayed, name);
    x = write_row_text(buf, x, y, max_x, &formatted_name, name_style);
    for column in columns {
        x = write_row_text(buf, x, y, max_x, COLUMN_SEPARATOR, sep_style);
        let cell_text = column.lay_out(column.cell(&displayed));
        x = write_row_text(buf, x, y, max_x, &cell_text, data_style);
    }
    x
}

fn write_spaces(buf: &mut Buffer, x: u16, y: u16, max_x: u16, count: usize, style: Style) -> u16 {
    let limit = usize::from(max_x.saturating_sub(x)).min(count);
    for offset in 0..limit {
        if let Some(cell) = buf.cell_mut((x + screen_u16(offset), y)) {
            cell.set_char(' ').set_style(style);
        }
    }
    x + screen_u16(limit)
}

fn write_row_text(buf: &mut Buffer, x: u16, y: u16, max_x: u16, text: &str, style: Style) -> u16 {
    if x >= max_x || text.is_empty() {
        return x;
    }
    if text == COLUMN_SEPARATOR {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(COLUMN_SEPARATOR).set_style(style);
        }
        return x + 1;
    }
    if is_printable_ascii(text) {
        let limit = usize::from(max_x - x).min(text.len());
        for (offset, byte) in text.bytes().take(limit).enumerate() {
            if let Some(cell) = buf.cell_mut((x + screen_u16(offset), y)) {
                cell.set_char(char::from(byte)).set_style(style);
            }
        }
        return x + screen_u16(limit);
    }
    let (next_x, _) = buf.set_stringn(x, y, text, usize::from(max_x - x), style);
    next_x
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
