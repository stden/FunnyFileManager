//! Purpose: Column fitting and row layout for the Network tab's upper
//! adapter throughput table.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::tasks::AdapterRow;

use super::super::super::panel::{center_cells, cursor_style, Mark};
use super::super::super::{palette, Cells};
use super::super::table::RowBar;
use super::adapter_cells::cell;
use super::separated;

const MIN_NAME: Cells = Cells::new(14);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NetCol {
    Name,
    Kind,
    State,
    Ipv4,
    RxRate,
    TxRate,
    TotalRx,
    TotalTx,
    Issues,
}

impl NetCol {
    const ALL: [Self; 9] = [
        Self::Name,
        Self::Kind,
        Self::State,
        Self::Ipv4,
        Self::RxRate,
        Self::TxRate,
        Self::TotalRx,
        Self::TotalTx,
        Self::Issues,
    ];
    const DROP_ORDER: [Self; 6] = [
        Self::TotalTx,
        Self::TotalRx,
        Self::Kind,
        Self::Issues,
        Self::Ipv4,
        Self::State,
    ];

    const fn fixed_width(self) -> usize {
        match self {
            Self::Name => 0,
            Self::Kind | Self::TotalRx | Self::TotalTx | Self::Issues => 8,
            Self::State => 5,
            Self::Ipv4 => 15,
            Self::RxRate | Self::TxRate => 9,
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Name => "Interface",
            Self::Kind => "Type",
            Self::State => "State",
            Self::Ipv4 => "IPv4",
            Self::RxRate => "RX/s",
            Self::TxRate => "TX/s",
            Self::TotalRx => "Total RX",
            Self::TotalTx => "Total TX",
            Self::Issues => "Err/Drop",
        }
    }
}

pub(super) struct AdapterColumns(Vec<(NetCol, Cells)>);

impl AdapterColumns {
    pub(super) fn fit(interior: Cells) -> Self {
        let mut shown = NetCol::ALL.to_vec();
        for dropped in NetCol::DROP_ORDER {
            if name_room(&shown, interior) >= MIN_NAME {
                break;
            }
            shown.retain(|c| *c != dropped);
        }
        let name = name_room(&shown, interior);
        let widths = shown.into_iter().map(|col| match col {
            NetCol::Name => (col, name),
            other => (other, Cells::new(other.fixed_width())),
        });
        Self(widths.collect())
    }

    pub(super) fn rule(&self, left: char, junction: char, right: char) -> String {
        let segments: Vec<String> = self.0.iter().map(|(_, w)| "─".repeat(w.get())).collect();
        format!("{left}{}{right}", segments.join(&junction.to_string()))
    }

    pub(super) fn titles(&self) -> Line<'static> {
        let style = Style::default().fg(palette::DATA);
        let cells = self
            .0
            .iter()
            .map(|&(col, w)| Span::styled(center_cells(col.title(), w), style));
        Line::from(separated(cells.collect()))
    }

    pub(super) fn row(&self, adapter: &AdapterRow, bar: RowBar) -> Line<'static> {
        let spans = separated(
            self.0
                .iter()
                .map(|&(col, w)| cell(adapter, col, w))
                .collect(),
        );
        match bar {
            RowBar::Cursor => {
                let style = cursor_style(Mark::Unmarked);
                Line::from(
                    spans
                        .into_iter()
                        .map(|s| s.patch_style(style))
                        .collect::<Vec<_>>(),
                )
            }
            RowBar::Plain => Line::from(spans),
        }
    }

    pub(super) fn empty(&self) -> Line<'static> {
        let style = Style::default().fg(palette::DATA);
        let cells = self
            .0
            .iter()
            .map(|(_, w)| Span::styled(" ".repeat(w.get()), style));
        Line::from(separated(cells.collect()))
    }
}

fn name_room(shown: &[NetCol], interior: Cells) -> Cells {
    let fixed: usize = shown.iter().map(|c| c.fixed_width()).sum();
    let seps = shown.len().saturating_sub(1);
    interior.saturating_sub(Cells::new(fixed.saturating_add(seps)))
}
