//! Purpose: Cell formatting and colouring for one row of the Network tab's
//! adapter throughput table.

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::models::tasks::{AdapterRow, BytesPerSec, InterfaceState};

use super::super::super::panel::{
    abbreviate_bytes, center_cells, clip_name, pad_cells, right_cells,
};
use super::super::super::{palette, printable_name, Cells};
use super::super::format;
use super::adapters::NetCol;

const BUSY_BYTES_PER_SEC: u64 = 1_048_576;

pub(super) fn cell(adapter: &AdapterRow, col: NetCol, width: Cells) -> Span<'static> {
    let s = adapter.sample();
    let data = Style::default().fg(palette::DATA);
    match col {
        NetCol::Name => {
            let fg = if s.state == InterfaceState::Down {
                palette::HIDDEN
            } else {
                palette::FILE
            };
            let text = pad_cells(&clip_name(&printable_name(&s.name), width), width);
            Span::styled(text, Style::default().fg(fg))
        }
        NetCol::Kind => Span::styled(pad_cells(s.kind.label(), width), data),
        NetCol::State => Span::styled(
            center_cells(s.state.label(), width),
            Style::default().fg(state_color(s.state)),
        ),
        NetCol::Ipv4 => {
            let ip = s
                .ipv4
                .map_or_else(|| format::UNKNOWN.to_owned(), |v| v.to_string());
            Span::styled(pad_cells(&ip, width), data)
        }
        NetCol::RxRate => rate_span(adapter.rx_rate(), width),
        NetCol::TxRate => rate_span(adapter.tx_rate(), width),
        NetCol::TotalRx => Span::styled(right_cells(&abbreviate_bytes(s.rx_bytes), width), data),
        NetCol::TotalTx => Span::styled(right_cells(&abbreviate_bytes(s.tx_bytes), width), data),
        NetCol::Issues => issues_span(adapter, width),
    }
}

const fn state_color(state: InterfaceState) -> Color {
    match state {
        InterfaceState::Up => palette::EXECUTABLE,
        InterfaceState::Dormant => palette::MARKED,
        InterfaceState::Down | InterfaceState::Unknown => palette::HIDDEN,
    }
}

fn rate_span(rate: Option<BytesPerSec>, width: Cells) -> Span<'static> {
    let fg = if rate.is_some_and(|r| r.bytes_per_sec() >= BUSY_BYTES_PER_SEC) {
        palette::EXECUTABLE
    } else {
        palette::DATA
    };
    Span::styled(
        right_cells(&format::rate(rate), width),
        Style::default().fg(fg),
    )
}

fn issues_span(adapter: &AdapterRow, width: Cells) -> Span<'static> {
    let errs = adapter.total_errors();
    let drops = adapter.total_drops();
    if errs == 0 && drops == 0 {
        Span::styled(right_cells("0", width), Style::default().fg(palette::DATA))
    } else {
        Span::styled(
            right_cells(&format!("{errs}/{drops}"), width),
            Style::default().fg(palette::PROMPT_FAILED),
        )
    }
}
