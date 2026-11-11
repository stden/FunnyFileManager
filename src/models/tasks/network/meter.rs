//! Purpose: Derives per-adapter throughput rows and the header's aggregate
//! `NetMeter` from two consecutive `NetworkSample` readings.

use std::collections::BTreeMap;
use std::time::Duration;

use super::types::{AdapterKind, AdapterSample, BytesPerSec, NetworkSample};
use super::AdapterFilter;

/// Minimum ceiling for a dynamic network bar (1 MiB/s) so tiny background
/// keepalives do not paint a 100% saturated bar when link speed is unknown.
const MIN_CEILING_BYTES_PER_SEC: u64 = 1_048_576;

/// Previous sample's network counters, keyed by adapter name.
#[derive(Debug, Clone, Default)]
pub struct NetBaselines {
    elapsed: Option<Duration>,
    counters: BTreeMap<String, (u64, u64)>,
}

/// One adapter's derived row in the `Snapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRow {
    sample: AdapterSample,
    rx_rate: Option<BytesPerSec>,
    tx_rate: Option<BytesPerSec>,
}

impl AdapterRow {
    #[must_use]
    pub const fn sample(&self) -> &AdapterSample {
        &self.sample
    }

    #[must_use]
    pub const fn rx_rate(&self) -> Option<BytesPerSec> {
        self.rx_rate
    }

    #[must_use]
    pub const fn tx_rate(&self) -> Option<BytesPerSec> {
        self.tx_rate
    }

    #[must_use]
    pub const fn total_errors(&self) -> u64 {
        self.sample.rx_errors.saturating_add(self.sample.tx_errors)
    }

    #[must_use]
    pub const fn total_drops(&self) -> u64 {
        self.sample.rx_drops.saturating_add(self.sample.tx_drops)
    }

    #[must_use]
    pub const fn is_shown(&self, filter: AdapterFilter) -> bool {
        match filter {
            AdapterFilter::All => true,
            AdapterFilter::Active => {
                !matches!(self.sample.kind, AdapterKind::Loopback)
                    && (self.sample.state.is_up()
                        || self.sample.rx_bytes > 0
                        || self.sample.tx_bytes > 0)
            }
        }
    }
}

/// Aggregate network meter shown in the top summary row when adapters exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetMeter {
    /// First sample: counters exist, rate awaits a second sample.
    Pending,
    /// Measured aggregate RX/TX rate across non-loopback adapters.
    Measured {
        rx: BytesPerSec,
        tx: BytesPerSec,
        ceiling: BytesPerSec,
        issues: u64,
    },
}

/// Derives adapter rows, optional summary `NetMeter`, and next baselines.
pub(in crate::models::tasks) fn derive_network(
    previous: &NetBaselines,
    sample: NetworkSample,
) -> (Vec<AdapterRow>, Option<NetMeter>, NetBaselines) {
    if sample.adapters.is_empty() {
        return (Vec::new(), None, NetBaselines::default());
    }
    let window = sample_window(previous.elapsed, sample.elapsed);
    let mut counters = BTreeMap::new();
    let mut rows = Vec::with_capacity(sample.adapters.len());
    for adapter in sample.adapters {
        counters.insert(adapter.name.clone(), (adapter.rx_bytes, adapter.tx_bytes));
        rows.push(derive_row(previous, window, adapter));
    }
    let meter = Some(aggregate_meter(&rows, window.is_some()));
    let next = NetBaselines {
        elapsed: sample.elapsed,
        counters,
    };
    (rows, meter, next)
}

fn sample_window(before: Option<Duration>, after: Option<Duration>) -> Option<Duration> {
    before
        .zip(after)
        .map(|(b, a)| a.saturating_sub(b))
        .filter(|dt| !dt.is_zero())
}

fn derive_row(
    previous: &NetBaselines,
    window: Option<Duration>,
    adapter: AdapterSample,
) -> AdapterRow {
    let (rx_rate, tx_rate) = match (window, previous.counters.get(&adapter.name)) {
        (Some(dt), Some(&(prev_rx, prev_tx))) => (
            BytesPerSec::between(prev_rx, adapter.rx_bytes, dt),
            BytesPerSec::between(prev_tx, adapter.tx_bytes, dt),
        ),
        _ => (None, None),
    };
    AdapterRow {
        sample: adapter,
        rx_rate,
        tx_rate,
    }
}

fn aggregate_meter(rows: &[AdapterRow], has_window: bool) -> NetMeter {
    if !has_window {
        return NetMeter::Pending;
    }
    let (mut rx_sum, mut tx_sum, mut link_max, mut issues) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut any_measured = false;
    for row in rows
        .iter()
        .filter(|r| !matches!(r.sample.kind, AdapterKind::Loopback))
    {
        if let (Some(rx), Some(tx)) = (row.rx_rate, row.tx_rate) {
            rx_sum = rx_sum.saturating_add(rx.bytes_per_sec());
            tx_sum = tx_sum.saturating_add(tx.bytes_per_sec());
            any_measured = true;
        }
        if let Some(bits) = row.sample.link_bps.filter(|_| row.sample.state.is_up()) {
            link_max = link_max.max(bits / 8);
        }
        issues = issues.saturating_add(row.total_errors().saturating_add(row.total_drops()));
    }
    if !any_measured {
        return NetMeter::Pending;
    }
    NetMeter::Measured {
        rx: BytesPerSec::from_bytes_per_sec(rx_sum),
        tx: BytesPerSec::from_bytes_per_sec(tx_sum),
        ceiling: meter_ceiling(rx_sum.saturating_add(tx_sum), link_max),
        issues,
    }
}

fn meter_ceiling(traffic: u64, link_bytes_max: u64) -> BytesPerSec {
    let base = if link_bytes_max > 0 {
        link_bytes_max.max(traffic)
    } else {
        traffic
            .max(MIN_CEILING_BYTES_PER_SEC)
            .checked_next_power_of_two()
            .unwrap_or(u64::MAX)
    };
    BytesPerSec::from_bytes_per_sec(base.max(1))
}
