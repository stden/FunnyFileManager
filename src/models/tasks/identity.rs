//! Purpose: Who a process is — a PID the kernel could hold, the stamp of when it
//! started, and the key made of both that tells a reused PID from the process
//! that held it before.
//!
//! The cursor, the marks, the frozen targets of a kill, the CPU baselines and
//! the command cache all name a process by [`ProcessKey`], so a PID handed to a
//! newcomer between two samples is a different key everywhere at once.

use std::fmt;
use std::num::NonZeroI32;

/// A process id: **1..=`i32::MAX`**.
///
/// `kill(0)` (the caller's group), `kill(-1)` (every process the caller may
/// signal) and `kill(-pgid)` (a group) have no representation, so no code
/// holding a `Pid` can ask for them. The platform layer is the only place that
/// turns one into a system-call argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid(NonZeroI32);

impl Pid {
    /// The PID `raw`, or `None` for 0 and for anything above `i32::MAX`, which
    /// a signed `pid_t` would read as a negative group id.
    #[must_use]
    pub fn new(raw: u32) -> Option<Self> {
        let signed = i32::try_from(raw).ok()?;
        NonZeroI32::new(signed).map(Self)
    }

    /// The PID as the unsigned number people read.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get().unsigned_abs()
    }

    /// The PID as the kernel's signed `pid_t`; always positive.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0.get()
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// When a process started, in its source's own unit: Linux `stat` field 22
/// (clock ticks since boot), a Windows creation `FILETIME`, or the epoch
/// seconds of `ps`'s `lstart`.
///
/// Opaque on purpose: two stamps from one source compare, and nothing else is
/// meaningful — there is no arithmetic and no display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StartTime(u64);

impl StartTime {
    /// The stamp `raw`, in the unit of the source that read it.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

/// Whether the source could say when a process started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Start {
    Known(StartTime),
    /// The source refused to say (Windows `GetProcessTimes` without access).
    /// Such a process cannot be told from a successor, so control of it is
    /// refused as unverifiable.
    Unknown,
}

/// The one identity of a process: its PID and when it started.
///
/// A PID reused by a newcomer is a different key, so nothing aimed at the
/// first process — a mark, a frozen kill target, a CPU baseline — carries over
/// to the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessKey {
    pid: Pid,
    start: Start,
}

impl ProcessKey {
    #[must_use]
    pub const fn new(pid: Pid, start: Start) -> Self {
        Self { pid, start }
    }

    #[must_use]
    pub const fn pid(self) -> Pid {
        self.pid
    }

    #[must_use]
    pub const fn start(self) -> Start {
        self.start
    }
}

/// One opening of the task list. A new session means a fresh sampling worker
/// and empty CPU baselines, so a result meant for an earlier opening is never
/// applied to a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSession(u64);

impl TaskSession {
    /// Session number `serial`. Only the host slot mints sessions, one per
    /// opening, so two openings never share a number.
    pub(super) const fn new(serial: u64) -> Self {
        Self(serial)
    }
}
