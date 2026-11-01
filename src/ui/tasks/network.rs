//! Purpose: Renders `TaskTab::Network` — the live per-adapter throughput table
//! on top and the non-blocking latency/DNS diagnostics section underneath.

mod adapter_cells;
mod adapters;
mod probe_cells;
mod probes;

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::models::{Table, TaskList};

use super::super::panel::{separator_style, COLUMN_SEPARATOR};
use super::super::{palette, Cells, FAR_BLUE};
use super::placement::TaskLayout;
use super::table::RowBar;
use adapters::AdapterColumns;
use probes::{section_lines, ProbeColumns};

/// Adapter rows drawn in `body` after reserving room for the diagnostics table.
pub(super) fn adapter_body_rows(body_height: u16, probe_count: usize) -> usize {
    let total = usize::from(body_height);
    if probe_count == 0 || total < 6 {
        return total;
    }
    let min_adapters = (total / 2).max(3);
    let max_diag = total.saturating_sub(min_adapters);
    let diag_rows = if max_diag >= 3 {
        probe_count.saturating_add(2).min(max_diag)
    } else {
        0
    };
    total.saturating_sub(diag_rows)
}

pub(super) fn render(f: &mut Frame<'_>, list: &TaskList, layout: &TaskLayout) {
    let interior = Cells::columns(layout.rows.titles.width);
    let cols = AdapterColumns::fit(interior);
    let probes = list.net_diagnostics();
    let adapter_rows = adapter_body_rows(layout.rows.body.height, probes.rows().len());
    let diag_rows = usize::from(layout.rows.body.height).saturating_sub(adapter_rows);
    let probe_cols = ProbeColumns::fit(interior);

    let edge = Style::default().fg(palette::FRAME).bg(FAR_BLUE);
    let top_rule = cols.rule('╟', '┬', '╢');
    let bottom_rule = if diag_rows >= 3 {
        probe_cols.rule('╟', '┴', '╢')
    } else {
        cols.rule('╟', '┴', '╢')
    };
    f.render_widget(Paragraph::new(top_rule).style(edge), layout.rows.meter_rule);
    f.render_widget(Paragraph::new(bottom_rule).style(edge), layout.rows.closing);
    f.render_widget(Paragraph::new(cols.titles()), layout.rows.titles);

    let mut lines = adapter_lines(list, layout, &cols, adapter_rows);
    if diag_rows >= 3 {
        lines.extend(section_lines(probes, &probe_cols, interior, diag_rows));
    }
    f.render_widget(Paragraph::new(lines), layout.rows.body);
}

fn adapter_lines(
    list: &TaskList,
    layout: &TaskLayout,
    cols: &AdapterColumns,
    rows: usize,
) -> Vec<Line<'static>> {
    if let Table::Failed(error) = list.table() {
        let sentence = Span::styled(error.to_string(), Style::default().fg(palette::DATA));
        return vec![Line::from(sentence)];
    }
    let adapters = list.visible_adapters();
    if adapters.is_empty() && list.snapshot().is_some() && rows > 0 {
        let msg = Span::styled(
            " No active network interfaces (press F2 for all)",
            Style::default().fg(palette::HIDDEN),
        );
        let mut out = vec![Line::from(msg)];
        out.extend(std::iter::repeat_with(|| cols.empty()).take(rows.saturating_sub(1)));
        return out;
    }
    let cursor = list.net_cursor();
    let shown = adapters.get(layout.window.clone()).unwrap_or_default();
    let drawn = shown.iter().zip(layout.window.clone()).map(|(row, idx)| {
        let bar = if cursor == Some(idx) {
            RowBar::Cursor
        } else {
            RowBar::Plain
        };
        cols.row(row, bar)
    });
    drawn
        .chain(std::iter::repeat_with(|| cols.empty()))
        .take(rows)
        .collect()
}

pub(super) fn separated(cells: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(cells.len().saturating_mul(2));
    for (idx, cell) in cells.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        }
        spans.push(cell);
    }
    spans
}
