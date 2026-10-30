//! Purpose: What a process source reads — raw counters, never shares — and the
//! two ports a platform implements: a host that signals and renices, and a
//! reader that samples on the worker thread.
//!
//! Sources report cumulative times and sizes as the system gives them; the
//! snapshot derives CPU %, the meters and the order once, for every platform.

mod process;

use std::time::Duration;

use super::{
    ControlError, LoadAverage, MemoryBytes, NetworkSample, PriorityChange, ProbeOutcome,
    ProbeTarget, ProcessKey, SampleError, Signal,
};

pub use process::{MemoryUsage, Owner, ProcessKind, ProcessSample, ProcessState, StateLetter};

/// One reading of the system and every process in it.
///
/// No `Debug`: the processes carry command lines, which may hold secrets.
#[derive(Clone, PartialEq, Eq)]
pub struct Sample {
    pub system: SystemSample,
    pub processes: Vec<ProcessSample>,
    /// Processes skipped because their files could not be read or parsed.
    pub unreadable: u32,
    pub network: NetworkSample,
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
    /// Optional latency and DNS prober run on a dedicated background worker.
    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        None
    }
}

/// Reads samples, one at a time, on the sampling worker.
pub trait ProcessReader: Send {
    fn sample(&mut self) -> Result<Sample, SampleError>;
}

/// Runs latency and DNS probes on a dedicated background worker so network
/// timeouts never delay process sampling or UI input.
pub trait NetProber: Send {
    fn probe(&mut self, targets: &[ProbeTarget]) -> Vec<(ProbeTarget, ProbeOutcome)>;
}
