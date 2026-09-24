//! Purpose: What a kill or a renice is aimed at — the processes frozen by
//! identity when the dialog opens — and the refusals decided before any
//! system call: FFM itself, init, kernel threads, and processes whose
//! identity cannot be confirmed.
//!
//! Freezing is the safety rule: refreshes continue behind a dialog and move
//! rows, but the targets are the identities the user saw, never whatever sits
//! under the cursor when Enter is pressed.

use std::fmt;

use super::tree::ancestors;
use super::{
    Niceness, Owner, Pid, ProcessKey, ProcessKind, ProcessRow, ProcessSample, Start, Table,
    TaskList,
};

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

/// Why a target is refused before any system call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRefusal {
    /// TERM would leave the terminal raw and STOP would freeze the screen.
    OwnProcess,
    /// SIGINT to systemd as PID 1 starts a reboot.
    Init,
    /// Kernel threads ignore signals.
    KernelThread,
    /// The source cannot tell this process from a successor.
    Unverifiable,
    /// A nice step would leave -20..=19.
    AtLimit(Niceness),
    /// A Windows Realtime class is never changed from here.
    RealtimeClass,
    /// A step would leave the Idle..=High classes.
    ClassAtLimit,
    /// The source could not read the priority, so there is nothing to step
    /// from.
    UnknownPriority,
    /// Nothing is marked and the list is empty.
    NoTarget,
}

impl fmt::Display for ControlRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OwnProcess => f.write_str("This is FFM itself; quit with F10."),
            Self::Init => f.write_str("PID 1 (init) is never signalled from here."),
            Self::KernelThread => f.write_str("Kernel threads ignore signals."),
            Self::Unverifiable => {
                f.write_str("FFM cannot confirm which process this is; nothing was sent.")
            }
            Self::AtLimit(nice) => write!(f, "Nice is already {}.", nice.get()),
            Self::RealtimeClass => f.write_str("The Realtime class is not changed from here."),
            Self::ClassAtLimit => f.write_str("The priority class is already at its limit."),
            Self::UnknownPriority => f.write_str("The priority of this process is not known."),
            Self::NoTarget => f.write_str("No process is selected."),
        }
    }
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

/// Why an operation left with no target does nothing: the first target's
/// refusal, or [`ControlRefusal::NoTarget`] when nothing was chosen at all.
pub(super) fn first_refusal(skipped: &[Skipped]) -> ControlRefusal {
    skipped
        .first()
        .map_or(ControlRefusal::NoTarget, Skipped::refusal)
}

/// Why `sample` is refused for `operation` when FFM's own PID is `own`, or
/// `None` when it may go ahead.
#[must_use]
pub(super) fn refusal(
    sample: &ProcessSample,
    own: Option<Pid>,
    operation: Operation,
) -> Option<ControlRefusal> {
    let for_signals = match operation {
        Operation::Signal => signal_refusal(sample, own),
        Operation::Renice => None,
    };
    let unknown = sample.key.start() == Start::Unknown;
    for_signals.or_else(|| unknown.then_some(ControlRefusal::Unverifiable))
}

/// The refusals only a signal has: FFM itself, init and kernel threads.
fn signal_refusal(sample: &ProcessSample, own: Option<Pid>) -> Option<ControlRefusal> {
    ControlRefusal::signal_to_pid(sample.key.pid(), own)
        .or_else(|| ControlRefusal::signal_to_kind(sample.kind))
}

impl ControlRefusal {
    /// Why no signal may go to `pid`, whatever runs there: it is FFM itself
    /// (`own`), or init. Asked with the PID alone, so the platform can ask
    /// before its first system call.
    #[must_use]
    pub fn signal_to_pid(pid: Pid, own: Option<Pid>) -> Option<Self> {
        if own == Some(pid) {
            Some(Self::OwnProcess)
        } else if pid.get() == INIT {
            Some(Self::Init)
        } else {
            None
        }
    }

    /// Why no signal may go to a process of `kind`: kernel threads ignore
    /// them.
    #[must_use]
    pub fn signal_to_kind(kind: ProcessKind) -> Option<Self> {
        match kind {
            ProcessKind::KernelThread => Some(Self::KernelThread),
            ProcessKind::User => None,
        }
    }
}

/// The PID of init, or of a container's first process.
const INIT: u32 = 1;

/// The rows `chosen` of `rows`, in that order, frozen for `operation`.
fn freeze(
    rows: &[ProcessRow],
    chosen: &[usize],
    own: Option<Pid>,
    operation: Operation,
) -> FrozenTargets {
    let above = own.map(|own| ancestors(rows, own)).unwrap_or_default();
    let mut frozen = FrozenTargets::default();
    for (row, process) in chosen
        .iter()
        .filter_map(|&row| rows.get(row).map(|process| (row, process)))
    {
        let sample = process.sample();
        let relation = if own == Some(sample.key.pid()) {
            Relation::Itself
        } else if above.contains(&row) {
            Relation::Ancestor
        } else {
            Relation::Other
        };
        let target = Target {
            key: sample.key,
            name: sample.name.clone(),
            owner: sample.owner.clone(),
            relation,
        };
        match refusal(sample, own, operation) {
            Some(refusal) => frozen.skipped.push(Skipped::new(target, refusal)),
            None => frozen.targets.push(target),
        }
    }
    frozen
}

impl TaskList {
    /// The targets of `operation` as they stand now: the marked shown
    /// processes in view order, or else the cursor's process.
    #[must_use]
    pub fn freeze_targets(&self, operation: Operation) -> FrozenTargets {
        let marked: Vec<usize> = self
            .view
            .iter()
            .filter(|shown| self.is_marked(shown.key()))
            .map(|shown| shown.row())
            .collect();
        let chosen = if marked.is_empty() {
            let cursor = self
                .cursor()
                .and_then(|cursor| self.view.get(cursor.index()));
            cursor.map(|shown| shown.row()).into_iter().collect()
        } else {
            marked
        };
        freeze(self.rows(), &chosen, self.own, operation)
    }
}

impl TaskList {
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
