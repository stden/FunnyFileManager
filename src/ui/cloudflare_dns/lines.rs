//! Purpose: The text the lists and the status line show, and the part of a list that fits the dialog.

use crate::models::cloudflare_dns::{Busy, CloudflareDnsPanel, Record, Zone};
use std::ops::Range;

/// The rows of a list of `len` that fit in `height`, kept around the cursor.
pub(super) fn window(len: usize, cursor: usize, height: usize) -> Range<usize> {
    let start = cursor
        .saturating_sub(height / 2)
        .min(len.saturating_sub(height));
    start..start.saturating_add(height).min(len)
}

pub(super) fn zone_line(zone: &Zone, marked: bool) -> String {
    format!("{}{}", marker(marked), zone.name)
}

pub(super) fn record_line(record: &Record, marked: bool) -> String {
    format!(
        "{}{:<6} {}  {}  TTL {}  proxied {}",
        marker(marked),
        record.kind,
        record.name,
        record.content,
        record.ttl.label(),
        record.proxy.label()
    )
}

/// What the status line says: the request in flight, or else the outcome of the last one.
pub(super) fn status(panel: &CloudflareDnsPanel) -> String {
    match panel.busy {
        Some(Busy::Zones) => "Loading zones...".to_owned(),
        Some(Busy::Records) => "Loading records...".to_owned(),
        Some(Busy::Host) => "Reading this host...".to_owned(),
        Some(Busy::Saving) => "Saving...".to_owned(),
        None => panel.status.clone(),
    }
}

fn marker(marked: bool) -> &'static str {
    if marked {
        "> "
    } else {
        "  "
    }
}
