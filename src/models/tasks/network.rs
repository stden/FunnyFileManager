//! Purpose: Network tab state and `TaskList` transitions for adapter browsing,
//! active/all filtering, and non-blocking latency/DNS diagnostics.

mod meter;
mod probes;
mod probes_schedule;
mod types;

use std::time::Instant;

use super::{ListMotion, TaskList};

pub(super) use meter::{derive_network, NetBaselines};
pub use meter::{AdapterRow, NetMeter};
pub use probes::{NetHealth, ProbeOutcome, ProbeRow, ProbeTarget};
pub use probes_schedule::{NetDiagnostics, PROBE_INTERVAL};
pub use types::{AdapterKind, AdapterSample, BytesPerSec, InterfaceState, Ipv4, NetworkSample};

/// Which main pane `TaskList` currently displays under its summary header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TaskTab {
    #[default]
    Processes,
    Network,
}

impl TaskTab {
    const fn toggled(self) -> Self {
        match self {
            Self::Processes => Self::Network,
            Self::Network => Self::Processes,
        }
    }
}

/// Which network interfaces are listed in the Network tab's adapter table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdapterFilter {
    /// Non-loopback interfaces that are UP or have transferred bytes.
    #[default]
    Active,
    /// Every interface reported by the OS, including loopback and down links.
    All,
}

impl AdapterFilter {
    const fn toggled(self) -> Self {
        match self {
            Self::Active => Self::All,
            Self::All => Self::Active,
        }
    }
}

/// Network-tab view state owned by `TaskList`.
#[derive(Debug, Clone, Default)]
pub(super) struct NetworkView {
    pub(super) tab: TaskTab,
    pub(super) filter: AdapterFilter,
    pub(super) cursor: usize,
    pub(super) offset: usize,
    pub(super) diagnostics: NetDiagnostics,
}

impl TaskList {
    #[must_use]
    pub const fn tab(&self) -> TaskTab {
        self.network.tab
    }

    pub fn toggle_tab(&mut self) {
        self.network.tab = self.network.tab.toggled();
        self.clamp_net_cursor();
    }

    #[must_use]
    pub const fn adapter_filter(&self) -> AdapterFilter {
        self.network.filter
    }

    pub fn toggle_adapter_filter(&mut self) {
        self.network.filter = self.network.filter.toggled();
        self.clamp_net_cursor();
    }

    /// Adapters matching the active `AdapterFilter`, in snapshot order.
    #[must_use]
    pub fn visible_adapters(&self) -> Vec<&AdapterRow> {
        let Some(snapshot) = self.snapshot() else {
            return Vec::new();
        };
        let filter = self.network.filter;
        snapshot
            .adapters()
            .iter()
            .filter(|row| row.is_shown(filter))
            .collect()
    }

    #[must_use]
    pub fn net_cursor(&self) -> Option<usize> {
        let len = self.visible_adapters().len();
        (len > 0).then(|| self.network.cursor.min(len - 1))
    }

    #[must_use]
    pub const fn net_offset(&self) -> usize {
        self.network.offset
    }

    #[must_use]
    pub fn selected_adapter(&self) -> Option<&AdapterRow> {
        let idx = self.net_cursor()?;
        self.visible_adapters().into_iter().nth(idx)
    }

    pub fn move_net_cursor(&mut self, motion: ListMotion, page: usize) {
        let len = self.visible_adapters().len();
        let Some(last) = len.checked_sub(1) else {
            self.network.cursor = 0;
            self.network.offset = 0;
            return;
        };
        let step = page.max(1);
        let current = self.network.cursor.min(last);
        let next = match motion {
            ListMotion::Up => current.saturating_sub(1),
            ListMotion::Down => current.saturating_add(1).min(last),
            ListMotion::PageUp => current.saturating_sub(step),
            ListMotion::PageDown => current.saturating_add(step).min(last),
            ListMotion::Home => 0,
            ListMotion::End => last,
        };
        self.select_net_index(next, step);
    }

    pub fn select_net_index(&mut self, index: usize, page: usize) {
        let len = self.visible_adapters().len();
        let Some(last) = len.checked_sub(1) else {
            self.network.cursor = 0;
            self.network.offset = 0;
            return;
        };
        let target = index.min(last);
        self.network.cursor = target;
        let span = page.max(1);
        if target < self.network.offset {
            self.network.offset = target;
        } else if target >= self.network.offset.saturating_add(span) {
            self.network.offset = target.saturating_add(1).saturating_sub(span);
        }
    }

    pub(super) fn clamp_net_cursor(&mut self) {
        let len = self.visible_adapters().len();
        if let Some(last) = len.checked_sub(1) {
            self.network.cursor = self.network.cursor.min(last);
            self.network.offset = self.network.offset.min(self.network.cursor);
        } else {
            self.network.cursor = 0;
            self.network.offset = 0;
        }
    }

    #[must_use]
    pub const fn net_diagnostics(&self) -> &NetDiagnostics {
        &self.network.diagnostics
    }

    /// Returns due probe targets for the background probe worker, if any.
    pub fn take_probe_request(&mut self, now: Instant) -> Option<Vec<ProbeTarget>> {
        self.network
            .diagnostics
            .take_due(now, self.refresh.cadence())
    }

    /// Applies completed latency / DNS probe outcomes from the worker.
    pub fn apply_probe_results(&mut self, results: &[(ProbeTarget, ProbeOutcome)], now: Instant) {
        self.network.diagnostics.record_results(results, now);
    }
}
