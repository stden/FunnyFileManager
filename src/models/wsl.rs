//! WSL dialog state and the typed requests interpreted by the application worker.

use std::path::PathBuf;

/// A WSL distribution name that can also be used as one UNC share component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionName(String);

impl DistributionName {
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        if name.is_empty()
            || name.trim() != name
            || matches!(name, "." | "..")
            || name.ends_with('.')
            || name
                .chars()
                .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        {
            return None;
        }
        Some(Self(name.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn root_path(&self) -> PathBuf {
        PathBuf::from(format!("\\\\wsl.localhost\\{}\\", self.0))
    }
}

/// Correlates a completion with the dialog that requested it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WslRequestId(u64);

impl WslRequestId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Commands contain validated domain values, never shell fragments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WslOperation {
    Refresh,
    Update,
    UpdatePackages(DistributionName),
    Open(DistributionName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WslOutput {
    Distributions(Vec<DistributionName>),
    Updated(String),
    PackagesUpdated(String),
    Root(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WslActivity {
    Ready,
    Busy {
        id: WslRequestId,
        operation: WslOperation,
    },
    Notice(String),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct WslDialog {
    distributions: Vec<DistributionName>,
    selected: Option<usize>,
    pub activity: WslActivity,
}

impl WslDialog {
    #[must_use]
    pub fn new(distributions: Vec<DistributionName>) -> Self {
        let selected = (!distributions.is_empty()).then_some(0);
        Self {
            distributions,
            selected,
            activity: WslActivity::Ready,
        }
    }

    #[must_use]
    pub fn distributions(&self) -> &[DistributionName] {
        &self.distributions
    }

    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    #[must_use]
    pub fn selected_distribution(&self) -> Option<&DistributionName> {
        self.selected.and_then(|i| self.distributions.get(i))
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.map(|i| i.saturating_sub(1));
    }

    pub fn move_down(&mut self) {
        self.selected = self
            .selected
            .map(|i| (i + 1).min(self.distributions.len() - 1));
    }

    pub fn first(&mut self) {
        self.selected = (!self.distributions.is_empty()).then_some(0);
    }

    pub fn last(&mut self) {
        self.selected = self.distributions.len().checked_sub(1);
    }

    pub(crate) fn replace_distributions(&mut self, distributions: Vec<DistributionName>) {
        let previous = self.selected_distribution().cloned();
        self.distributions = distributions;
        self.selected = previous
            .and_then(|name| self.distributions.iter().position(|d| *d == name))
            .or_else(|| (!self.distributions.is_empty()).then_some(0));
    }
}

#[derive(Debug, Clone)]
pub struct WslRequest {
    pub id: WslRequestId,
    pub operation: WslOperation,
}

/// State-only outbox. Cloning App never replays a pending system command.
#[derive(Debug, Default)]
pub struct WslRequests {
    sequence: u64,
    pending: Vec<WslRequest>,
}

impl Clone for WslRequests {
    fn clone(&self) -> Self {
        Self {
            sequence: self.sequence,
            pending: Vec::new(),
        }
    }
}

impl WslRequests {
    #[must_use]
    pub fn pending(&self) -> &[WslRequest] {
        &self.pending
    }

    pub(crate) fn enqueue(&mut self, operation: WslOperation) -> WslRequestId {
        self.sequence = self.sequence.wrapping_add(1);
        let id = WslRequestId(self.sequence);
        self.pending.push(WslRequest { id, operation });
        id
    }

    pub(crate) fn take(&mut self) -> Vec<WslRequest> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests;
