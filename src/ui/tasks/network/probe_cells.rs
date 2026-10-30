//! Purpose: Cell formatting and integer RTT rendering for `ProbeRow` cells in
//! the Network tab's Latency & DNS Diagnostics table.

use std::time::Duration;

use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::models::tasks::{NetHealth, ProbeOutcome, ProbeRow, ProbeTarget};

use super::super::super::panel::{center_cells, clip_name, pad_cells, right_cells};
use super::super::super::{palette, Cells};
use super::super::format;
use super::probes::ProbeCol;

pub(super) fn probe_cell(probe: &ProbeRow, col: ProbeCol, width: Cells) -> Span<'static> {
    let target = probe.target();
    let health = probe.health();
    let data = Style::default().fg(palette::DATA);
    match col {
        ProbeCol::Target => Span::styled(
            pad_cells(&clip_name(&probe_name(target), width), width),
            Style::default().fg(palette::FILE),
        ),
        ProbeCol::Address => Span::styled(pad_cells(&target.ip().to_string(), width), data),
        ProbeCol::Check => Span::styled(center_cells(target.kind_label(), width), data),
        ProbeCol::LastRtt => Span::styled(right_cells(&last_text(probe.last()), width), data),
        ProbeCol::AvgRtt => {
            let text = probe
                .avg_rtt()
                .map_or_else(|| format::UNKNOWN.to_owned(), rtt_text);
            Span::styled(right_cells(&text, width), data)
        }
        ProbeCol::Loss => loss_span(probe, width),
        ProbeCol::Status => Span::styled(
            center_cells(health.label(), width),
            Style::default().fg(health_color(health)),
        ),
    }
}

fn loss_span(probe: &ProbeRow, width: Cells) -> Span<'static> {
    let pct = probe.loss_percent();
    let color = if pct.hundredths() > 0 {
        palette::PROMPT_FAILED
    } else {
        palette::DATA
    };
    let text = format!("{}%", format::percent(pct));
    Span::styled(right_cells(&text, width), Style::default().fg(color))
}

fn probe_name(target: ProbeTarget) -> String {
    match target {
        ProbeTarget::Gateway(_) => "Default gateway".to_owned(),
        ProbeTarget::SystemDns(_) => "System DNS".to_owned(),
        ProbeTarget::PublicDns { name, .. } => name.to_owned(),
        ProbeTarget::DnsResolve { domain, .. } => format!("DNS {domain}"),
    }
}

fn last_text(last: Option<ProbeOutcome>) -> String {
    match last {
        None => format::UNKNOWN.to_owned(),
        Some(ProbeOutcome::Ok(rtt)) => rtt_text(rtt),
        Some(ProbeOutcome::Timeout) => "timeout".to_owned(),
        Some(ProbeOutcome::Unreachable) => "unreach".to_owned(),
    }
}

fn rtt_text(rtt: Duration) -> String {
    let micros = rtt.as_micros();
    if micros < 10_000 {
        let tenths = (micros + 50) / 100;
        format!("{}.{} ms", tenths / 10, tenths % 10)
    } else {
        format!("{} ms", (micros + 500) / 1_000)
    }
}

const fn health_color(health: NetHealth) -> Color {
    match health {
        NetHealth::Testing => palette::HIDDEN,
        NetHealth::Healthy => palette::EXECUTABLE,
        NetHealth::Slow => palette::MARKED,
        NetHealth::Lossy | NetHealth::Down => palette::PROMPT_FAILED,
    }
}
