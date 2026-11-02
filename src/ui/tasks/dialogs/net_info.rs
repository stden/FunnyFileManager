//! Purpose: The task list's network interface dialog (`F3` / `Enter` on
//! `TaskTab::Network`) — displays full counters, link speed, and latency/DNS
//! summary for the selected adapter without performing any I/O.

use ratatui::Frame;

use crate::models::tasks::{AdapterRow, NetDiagnostics, ProbeOutcome};
use crate::models::TaskList;
use crate::mouse::DialogMouse;

use super::super::super::dialog::{render_tracked, Button, Ground, Row};
use super::super::super::panel::{abbreviate_bytes, clip_name, group_digits, pad_cells};
use super::super::super::{printable_name, Cells};
use super::super::format;

const LABEL: Cells = Cells::new(15);
const VALUE_WRAP: Cells = Cells::new(48);

pub(super) fn net_info(f: &mut Frame<'_>, tracker: &mut DialogMouse, list: &TaskList) {
    let mut lines = list.selected_adapter().map_or_else(
        || vec!["No network interface selected.".to_owned()],
        adapter_lines,
    );
    lines.push(diagnostics_line(list.net_diagnostics()));
    let rows: Vec<Row<'_>> = lines.iter().map(|line| Row::Plain(line)).collect();
    let buttons = [Button::new("OK").default_button().focused(true)];
    render_tracked(
        f,
        tracker,
        "Network interface",
        Ground::Prompt,
        &rows,
        &buttons,
    );
}

fn adapter_lines(adapter: &AdapterRow) -> Vec<String> {
    let s = adapter.sample();
    let ip = s
        .ipv4
        .map_or_else(|| format::UNKNOWN.to_owned(), |v| v.to_string());
    let link = s.link_bps.map_or_else(
        || format::UNKNOWN.to_owned(),
        |bps| format!("{} Mbps", bps / 1_000_000),
    );
    let desc = clip_name(&printable_name(&s.description), VALUE_WRAP);
    let mut lines = vec![
        field("Interface", &printable_name(&s.name)),
        field("Description", &desc),
        field(
            "Type / State",
            &format!("{} / {}", s.kind.label(), s.state.label()),
        ),
        field("IPv4", &ip),
        field("Link speed", &link),
    ];
    lines.extend(counter_lines(adapter));
    lines
}

fn counter_lines(adapter: &AdapterRow) -> [String; 3] {
    let s = adapter.sample();
    let rates = format!(
        "RX {}  TX {}",
        format::rate(adapter.rx_rate()),
        format::rate(adapter.tx_rate())
    );
    let totals = format!(
        "RX {} ({} pkts)  TX {} ({} pkts)",
        abbreviate_bytes(s.rx_bytes),
        group_digits(s.rx_packets),
        abbreviate_bytes(s.tx_bytes),
        group_digits(s.tx_packets)
    );
    let issues = format!(
        "RX {} err, {} drop  TX {} err, {} drop",
        s.rx_errors, s.rx_drops, s.tx_errors, s.tx_drops
    );
    [
        field("Throughput", &rates),
        field("Total traffic", &totals),
        field("Errors / Drops", &issues),
    ]
}

fn diagnostics_line(diag: &NetDiagnostics) -> String {
    let overall = diag.overall_health().label();
    let total = diag.rows().len();
    let ok = diag
        .rows()
        .iter()
        .filter(|row| row.last().is_some_and(|o| matches!(o, ProbeOutcome::Ok(_))))
        .count();
    field(
        "Connectivity",
        &format!("{overall} ({ok}/{total} checks OK)"),
    )
}

fn field(label: &str, value: &str) -> String {
    format!("{}{value}", pad_cells(label, LABEL))
}
