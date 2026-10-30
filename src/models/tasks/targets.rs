//! Purpose: What a kill or a renice is aimed at — the processes frozen by
//! identity when the dialog opens — and the refusals decided before any
//! system call: FFM itself, init, kernel threads, and processes whose
//! identity cannot be confirmed.
//!
//! Freezing is the safety rule: refreshes continue behind a dialog and move
//! rows, but the targets are the identities the user saw, never whatever sits
//! under the cursor when Enter is pressed.

mod freeze;
mod refusals;

use super::{Niceness, Owner, PriorityChange, ProcessKey, Table, TaskList};

pub use refusals::ControlRefusal;
pub(super) use refusals::{first_refusal, refusal};

/// A process a dialog is about, frozen when it opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    key: ProcessKey,
    /// The kernel's short name, untrusted text; never the command line.
    name: String,
    /// Named when a signal is refused as another user's.
    owner: Owner,
    relation: Relation,
}

impl Target {
    #[must_use]
    pub const fn key(&self) -> ProcessKey {
        self.key
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn owner(&self) -> &Owner {
        &self.owner
    }

    #[must_use]
    pub const fn relation(&self) -> Relation {
        self.relation
    }
}

/// How a target is related to FFM's own process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// FFM itself.
    Itself,
    /// A process FFM runs inside — the shell, tmux, sshd, the terminal:
    /// ending it ends this session, so the confirmation says so.
    Ancestor,
    Other,
}

/// A target left out of an operation, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    target: Target,
    refusal: ControlRefusal,
}

impl Skipped {
    pub(super) const fn new(target: Target, refusal: ControlRefusal) -> Self {
        Self { target, refusal }
    }

    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }

    #[must_use]
    pub const fn refusal(&self) -> ControlRefusal {
        self.refusal
    }
}

/// What is being done to the targets; the refusals differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Signal,
    /// Renicing FFM itself, init or a kernel thread is harmless and allowed.
    Renice,
}

/// The processes a dialog acts on, and those it refused, in view order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrozenTargets {
    targets: Vec<Target>,
    skipped: Vec<Skipped>,
}

impl FrozenTargets {
    #[must_use]
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    #[must_use]
    pub fn skipped(&self) -> &[Skipped] {
        &self.skipped
    }

    /// Why nothing can be done: `None` while a target remains; otherwise
    /// the first refusal, or [`ControlRefusal::NoTarget`] when nothing was
    /// chosen at all.
    #[must_use]
    pub fn refusal(&self) -> Option<ControlRefusal> {
        self.targets
            .is_empty()
            .then(|| first_refusal(&self.skipped))
    }
}

impl TaskList {
    /// A priority change of `key` was accepted: the table shows it until the
    /// next sample, and the next step starts from it rather than a stale read.
    pub fn record_priority(&mut self, key: ProcessKey, change: PriorityChange) {
        if let Table::Ready { snapshot, .. } = &mut self.table {
            snapshot.record_priority(key, change);
        }
        self.rebuild();
    }

    /// A renice of `key` to `nice` was accepted: the table shows it until the
    /// next sample, and the next nice step starts from it rather than from a
    /// stale reading.
    pub fn record_nice(&mut self, key: ProcessKey, nice: Niceness) {
        if let Table::Ready { snapshot, .. } = &mut self.table {
            snapshot.record_nice(key, nice);
        }
        self.rebuild();
    }
}
