//! Purpose: Network-tab chrome helpers — footer counts/health, cursor adapter
//! info line, and key-bar labels while viewing `TaskTab::Network`.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::models::tasks::AdapterFilter;
use crate::models::TaskList;

use super::super::super::fkey_bar::FKEY_COUNT;
use super::super::super::panel::clip_name;
use super::super::super::{palette, printable_name, Cells};
use super::super::format;

pub(super) fn footer(list: &TaskList) -> Option<String> {
    let snapshot = list.snapshot()?;
    let total = snapshot.adapters().len();
    let shown = list.visible_adapters().len();
    let count = if shown == total {
        adapters(total)
    } else {
        format!("{shown} of {}", adapters(total))
    };
    let health = list.net_diagnostics().overall_health().label();
    Some(format!(" {count}, net: {health} "))
}

fn adapters(count: usize) -> String {
    match count {
        1 => "1 adapter".to_owned(),
        count => format!("{count} adapters"),
    }
}

pub(super) fn info(list: &TaskList, width: Cells) -> Line<'static> {
    let Some(adapter) = list.selected_adapter() else {
        return Line::default();
    };
    let s = adapter.sample();
    let ip = s
        .ipv4
        .map_or_else(|| format::UNKNOWN.to_owned(), |ip| ip.to_string());
    let name = printable_name(&s.name);
    let desc = printable_name(&s.description);
    let text = if desc.is_empty() || desc == name {
        format!("{name} ({}, {}, IP {ip})", s.kind.label(), s.state.label())
    } else {
        format!(
            "{name} ({}, {}, IP {ip}): {desc}",
            s.kind.label(),
            s.state.label()
        )
    };
    Line::from(Span::styled(
        clip_name(&text, width),
        Style::default().fg(palette::FILE),
    ))
}

pub(super) const fn key_labels(
    list: &TaskList,
    hold: &'static str,
) -> [(&'static str, &'static str); FKEY_COUNT] {
    let filter = match list.adapter_filter() {
        AdapterFilter::Active => "All",
        AdapterFilter::All => "Active",
    };
    [
        ("1", "Help"),
        ("2", filter),
        ("3", "Info"),
        ("4", ""),
        ("5", ""),
        ("6", ""),
        ("7", ""),
        ("8", ""),
        ("9", ""),
        ("10", "Quit"),
        ("11", hold),
        ("12", "Proc"),
    ]
}
