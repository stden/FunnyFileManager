//! Purpose: Layout and section lines of the Network tab's lower Latency & DNS
//! Diagnostics table.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::tasks::{NetDiagnostics, ProbeRow};

use super::super::super::panel::{center_cells, clip_name};
use super::super::super::{palette, Cells};
use super::probe_cells::probe_cell;
use super::separated;

const MIN_TARGET: Cells = Cells::new(14);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ProbeCol {
    Target,
    Address,
    Check,
    LastRtt,
    AvgRtt,
    Loss,
    Status,
}

impl ProbeCol {
    const ALL: [Self; 7] = [
        Self::Target,
        Self::Address,
        Self::Check,
        Self::LastRtt,
        Self::AvgRtt,
        Self::Loss,
        Self::Status,
    ];
    const DROP_ORDER: [Self; 4] = [Self::Check, Self::AvgRtt, Self::Address, Self::Loss];

    pub(super) const fn fixed_width(self) -> usize {
        match self {
            Self::Target => 0,
            Self::Address => 15,
            Self::Check | Self::Loss => 6,
            Self::LastRtt | Self::AvgRtt => 9,
            Self::Status => 8,
        }
    }

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Target => "Target",
            Self::Address => "Address",
            Self::Check => "Check",
            Self::LastRtt => "Last RTT",
            Self::AvgRtt => "Avg RTT",
            Self::Loss => "Loss",
            Self::Status => "Status",
        }
    }
}

pub(super) struct ProbeColumns(Vec<(ProbeCol, Cells)>);

impl ProbeColumns {
    pub(super) fn fit(interior: Cells) -> Self {
        let mut shown = ProbeCol::ALL.to_vec();
        for dropped in ProbeCol::DROP_ORDER {
            if target_room(&shown, interior) >= MIN_TARGET {
                break;
            }
            shown.retain(|c| *c != dropped);
        }
        let target = target_room(&shown, interior);
        let widths = shown.into_iter().map(|col| match col {
            ProbeCol::Target => (col, target),
            other => (other, Cells::new(other.fixed_width())),
        });
        Self(widths.collect())
    }

    pub(super) fn rule(&self, left: char, junction: char, right: char) -> String {
        let segments: Vec<String> = self.0.iter().map(|(_, w)| "─".repeat(w.get())).collect();
        format!("{left}{}{right}", segments.join(&junction.to_string()))
    }

    fn titles(&self) -> Line<'static> {
        let style = Style::default().fg(palette::DATA);
        let cells = self
            .0
            .iter()
            .map(|&(col, w)| Span::styled(center_cells(col.title(), w), style));
        Line::from(separated(cells.collect()))
    }

    fn row(&self, probe: &ProbeRow) -> Line<'static> {
        let cells = self.0.iter().map(|&(col, w)| probe_cell(probe, col, w));
        Line::from(separated(cells.collect()))
    }

    fn empty(&self) -> Line<'static> {
        let style = Style::default().fg(palette::DATA);
        let cells = self
            .0
            .iter()
            .map(|(_, w)| Span::styled(" ".repeat(w.get()), style));
        Line::from(separated(cells.collect()))
    }
}

pub(super) fn section_lines(
    diag: &NetDiagnostics,
    columns: &ProbeColumns,
    interior: Cells,
    rows: usize,
) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(rows);
    out.push(divider_line(interior));
    if rows >= 2 {
        out.push(columns.titles());
    }
    let body_budget = rows.saturating_sub(2);
    for probe in diag.rows().iter().take(body_budget) {
        out.push(columns.row(probe));
    }
    while out.len() < rows {
        out.push(columns.empty());
    }
    out
}

fn divider_line(width: Cells) -> Line<'static> {
    let caption = " Latency & DNS Diagnostics ";
    let cap_cells = Cells::of(caption).get();
    let total = width.get();
    let text = if total <= cap_cells {
        clip_name(caption, width)
    } else {
        let dashes = total.saturating_sub(cap_cells);
        let left = dashes / 2;
        format!("{}{caption}{}", "─".repeat(left), "─".repeat(dashes - left))
    };
    Line::from(Span::styled(text, Style::default().fg(palette::FRAME)))
}

fn target_room(shown: &[ProbeCol], interior: Cells) -> Cells {
    let fixed: usize = shown.iter().map(|c| c.fixed_width()).sum();
    let seps = shown.len().saturating_sub(1);
    interior.saturating_sub(Cells::new(fixed.saturating_add(seps)))
}
