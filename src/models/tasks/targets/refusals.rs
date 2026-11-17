//! Purpose: Why a target is refused before any system call — FFM itself,
//! init, a kernel thread, an identity that cannot be confirmed, a priority at
//! its limit — and the rules that decide it.

use std::fmt;

use super::super::{Niceness, Pid, ProcessKind, ProcessSample, Start};
use super::{Operation, Skipped};

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

/// Why an operation left with no target does nothing: the first target's
/// refusal, or [`ControlRefusal::NoTarget`] when nothing was chosen at all.
pub(in crate::models::tasks) fn first_refusal(skipped: &[Skipped]) -> ControlRefusal {
    skipped
        .first()
        .map_or(ControlRefusal::NoTarget, Skipped::refusal)
}

/// Why `sample` is refused for `operation` when FFM's own PID is `own`, or
/// `None` when it may go ahead.
#[must_use]
pub(in crate::models::tasks) fn refusal(
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
