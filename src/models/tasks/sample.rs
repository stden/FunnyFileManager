//! Purpose: What a process source reads — raw counters, never shares — and the
//! two ports a platform implements: a host that signals and renices, and a
//! reader that samples on the worker thread.
//!
//! Sources report cumulative times and sizes as the system gives them; the
//! snapshot derives CPU %, the meters and the order once, for every platform.

use std::time::Duration;

use super::{
    ControlError, LoadAverage, MemoryBytes, Pid, Priority, PriorityChange, ProcessKey, SampleError,
    Signal,
};

/// One reading of the system and every process in it.
///
/// No `Debug`: the processes carry command lines, which may hold secrets.
#[derive(Clone, PartialEq, Eq)]
pub struct Sample {
    pub system: SystemSample,
    pub processes: Vec<ProcessSample>,
    /// Processes skipped because their files could not be read or parsed.
    pub unreadable: u32,
}

/// The system-wide counters of one sample. Everything but the CPU times is
/// optional: a source that cannot read one only loses that meter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSample {
    /// Every CPU together.
    pub total: CpuCounters,
    /// One entry per online core, in index order.
    pub cores: Vec<CoreSample>,
    pub memory: Option<MemorySample>,
    pub swap: Option<SwapSample>,
    pub load: Option<LoadAverage>,
    pub uptime: Option<Duration>,
    /// Runnable tasks at the moment of the sample.
    pub running: Option<u32>,
}

/// Cumulative CPU time since boot, already folded into the five parts a meter
/// draws: guest time is inside `virtualised` and nowhere else, I/O wait is
/// idle, and interrupts are system time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuCounters {
    pub user: Duration,
    pub nice: Duration,
    pub system: Duration,
    pub idle: Duration,
    pub virtualised: Duration,
}

/// One online core's counters. `index` is the core's own number, because
/// offline cores leave gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreSample {
    pub index: u16,
    pub counters: CpuCounters,
}

/// System memory as the kernel accounts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemorySample {
    pub total: MemoryBytes,
    pub free: MemoryBytes,
    pub available: MemoryBytes,
    pub buffers: MemoryBytes,
    pub cached: MemoryBytes,
    /// Reclaimable kernel slab, counted as cache.
    pub reclaimable: MemoryBytes,
    /// Shared memory (`tmpfs`, `shm`), which is inside `cached`.
    pub shared: MemoryBytes,
}

/// Swap space as the kernel accounts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapSample {
    pub total: MemoryBytes,
    pub free: MemoryBytes,
    /// Pages held both in swap and in memory.
    pub cached: MemoryBytes,
}

/// One process as read, before any share is derived.
///
/// No `Debug`: `command` is the full command line, which may hold a secret.
#[derive(Clone, PartialEq, Eq)]
pub struct ProcessSample {
    pub key: ProcessKey,
    /// `None` for a process with no parent (PID 1, kernel roots).
    pub parent: Option<Pid>,
    /// The short name the kernel keeps (`comm`), untrusted text.
    pub name: String,
    /// The command line, or `[name]` when there is none; untrusted text.
    pub command: String,
    pub owner: Owner,
    pub state: ProcessState,
    pub priority: Priority,
    pub threads: u32,
    /// User plus system time used since the process started.
    pub cpu_time: Duration,
    pub memory: Option<MemoryUsage>,
    pub running_for: Option<Duration>,
    pub kind: ProcessKind,
}

/// A process's memory, in the sizes htop shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryUsage {
    pub virtual_size: MemoryBytes,
    pub resident: MemoryBytes,
    /// `None` where the platform does not report it.
    pub shared: Option<MemoryBytes>,
}

/// Whose a process is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    Named(String),
    /// A uid with no passwd entry, shown as its number.
    Uid(u32),
    /// The source could not read the owner.
    Unknown,
}

/// What a process is doing, by the kernel's state letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    /// Uninterruptible sleep, usually disk I/O (`D`).
    DiskSleep,
    Zombie,
    Stopped,
    TracingStop,
    Dead,
    Idle,
    Parked,
    /// A letter this list does not name.
    Other(StateLetter),
}

impl ProcessState {
    /// The state for the kernel's letter. Total: an unknown letter is kept as
    /// [`ProcessState::Other`].
    #[must_use]
    pub const fn from_letter(letter: u8) -> Self {
        match letter {
            b'R' => Self::Running,
            b'S' => Self::Sleeping,
            b'D' => Self::DiskSleep,
            b'Z' => Self::Zombie,
            b'T' => Self::Stopped,
            b't' => Self::TracingStop,
            b'X' => Self::Dead,
            b'I' => Self::Idle,
            b'P' => Self::Parked,
            other => Self::Other(StateLetter::new(other)),
        }
    }

    /// The letter shown in the `S` column: always a visible ASCII character.
    #[must_use]
    pub fn letter(self) -> char {
        let byte = match self {
            Self::Running => b'R',
            Self::Sleeping => b'S',
            Self::DiskSleep => b'D',
            Self::Zombie => b'Z',
            Self::Stopped => b'T',
            Self::TracingStop => b't',
            Self::Dead => b'X',
            Self::Idle => b'I',
            Self::Parked => b'P',
            Self::Other(StateLetter(byte)) => byte,
        };
        char::from(byte)
    }
}

/// An unnamed state letter, safe to draw.
///
/// It is a visible ASCII character, or `?` in place of anything else: the
/// letter comes from a file another process can influence, so a raw control
/// byte never reaches the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateLetter(u8);

impl StateLetter {
    const fn new(byte: u8) -> Self {
        if byte.is_ascii_graphic() {
            Self(byte)
        } else {
            Self(b'?')
        }
    }
}

/// Whether a process runs user code or is a kernel thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    User,
    KernelThread,
}

/// A platform's process table: it opens readers and controls processes.
///
/// Used on the UI thread only; only the reader moves to the worker. It is
/// still `Send + Sync`, because `App` shares it through an `Arc`, and a
/// host that wraps another — replay's read-only one — is shared the same way.
pub trait ProcessHost: Send + Sync {
    /// A reader for one task-list session.
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError>;
    /// Sends `signal` to the process `key` names, if it is still that process.
    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError>;
    /// Changes the priority of the process `key` names, if it is still that
    /// process.
    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError>;
}

/// Reads samples, one at a time, on the sampling worker.
pub trait ProcessReader: Send {
    fn sample(&mut self) -> Result<Sample, SampleError>;
}
