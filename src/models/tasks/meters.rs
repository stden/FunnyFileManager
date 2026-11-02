//! Purpose: The header meters of one snapshot, derived from two samples'
//! counters: CPU bars, memory, swap and network bars, and the task counts.
//!
//! Integer maths only. Every bar's segments are built so that they never add
//! up to more than the whole bar, whatever the counters say: a source can
//! report inconsistent values, and a meter must still draw.

mod cpu;
mod memory;

use std::time::Duration;

use super::{LoadAverage, NetMeter, ProcessKind, ProcessSample, SystemSample};

pub(super) use cpu::{total_time, SystemCounters};
pub use cpu::{CoreGroup, CpuBar, CpuMeter, CpuMeters};
pub use memory::{MemoryMeter, SwapMeter};

/// Every header meter of one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meters {
    cpu: CpuMeter,
    cores: CpuMeters,
    memory: Option<MemoryMeter>,
    swap: Option<SwapMeter>,
    network: Option<NetMeter>,
    tasks: TaskCounts,
    load: Option<LoadAverage>,
    uptime: Option<Duration>,
}

impl Meters {
    /// The meters for `system`, measured against the counters of the sample
    /// before it (`None` for a session's first sample).
    pub(super) fn derive(
        previous: Option<&SystemCounters>,
        system: &SystemSample,
        network: Option<NetMeter>,
        tasks: TaskCounts,
    ) -> Self {
        Self {
            cpu: CpuMeter::total(previous, system),
            cores: CpuMeters::per_core(previous, system),
            memory: system.memory.as_ref().map(MemoryMeter::from_sample),
            swap: system.swap.as_ref().map(SwapMeter::from_sample),
            network,
            tasks,
            load: system.load,
            uptime: system.uptime,
        }
    }

    /// All CPUs together.
    #[must_use]
    pub const fn cpu(&self) -> CpuMeter {
        self.cpu
    }

    #[must_use]
    pub const fn cores(&self) -> &CpuMeters {
        &self.cores
    }

    #[must_use]
    pub const fn memory(&self) -> Option<MemoryMeter> {
        self.memory
    }

    #[must_use]
    pub const fn swap(&self) -> Option<SwapMeter> {
        self.swap
    }

    #[must_use]
    pub const fn network(&self) -> Option<NetMeter> {
        self.network
    }

    #[must_use]
    pub const fn tasks(&self) -> TaskCounts {
        self.tasks
    }

    #[must_use]
    pub const fn load(&self) -> Option<LoadAverage> {
        self.load
    }

    #[must_use]
    pub const fn uptime(&self) -> Option<Duration> {
        self.uptime
    }
}

/// The `Tasks:` line: user processes, their threads beyond the first, kernel
/// threads, and the runnable count when the source gives one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskCounts {
    processes: u32,
    threads: u32,
    kernel_threads: u32,
    running: Option<u32>,
}

impl TaskCounts {
    pub(super) fn of<'a>(
        processes: impl IntoIterator<Item = &'a ProcessSample>,
        running: Option<u32>,
    ) -> Self {
        let (users, threads, kernel_threads) = processes.into_iter().fold(
            (0_u32, 0_u32, 0_u32),
            |(users, threads, kernel), process| match process.kind {
                ProcessKind::User => (
                    users.saturating_add(1),
                    threads.saturating_add(process.threads),
                    kernel,
                ),
                ProcessKind::KernelThread => (users, threads, kernel.saturating_add(1)),
            },
        );
        Self {
            processes: users,
            threads: threads.saturating_sub(users),
            kernel_threads,
            running,
        }
    }

    #[must_use]
    pub const fn processes(self) -> u32 {
        self.processes
    }

    /// Threads of user processes beyond each process's first.
    #[must_use]
    pub const fn threads(self) -> u32 {
        self.threads
    }

    #[must_use]
    pub const fn kernel_threads(self) -> u32 {
        self.kernel_threads
    }

    #[must_use]
    pub const fn running(self) -> Option<u32> {
        self.running
    }
}
