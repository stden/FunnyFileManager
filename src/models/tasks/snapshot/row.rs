//! Purpose: One row of a snapshot — a process as sampled, with its CPU and
//! memory shares — and the CPU time between two samples those shares divide.

use std::num::NonZeroU128;
use std::time::Duration;

use super::super::meters::{total_time, SystemCounters};
use super::super::{CpuUsage, MemoryBytes, Percent, ProcessSample, SystemSample};

/// One process as sampled, with the shares derived for it.
///
/// No `Debug`, for the same reason as [`Snapshot`](super::Snapshot).
#[derive(Clone)]
pub struct ProcessRow {
    pub(super) sample: ProcessSample,
    cpu: CpuUsage,
    memory_share: Option<Percent>,
}

impl ProcessRow {
    pub(super) fn derive(
        sample: ProcessSample,
        cpu_time_before: Option<Duration>,
        window: &CpuWindow,
        memory_total: Option<MemoryBytes>,
    ) -> Self {
        let cpu = window.usage(sample.cpu_time, cpu_time_before);
        let resident = sample.memory.map(|memory| memory.resident);
        Self {
            sample,
            cpu,
            memory_share: memory_share(resident, memory_total),
        }
    }

    #[must_use]
    pub const fn sample(&self) -> &ProcessSample {
        &self.sample
    }

    /// The CPU share since the previous sample: `CPU%`.
    #[must_use]
    pub const fn cpu(&self) -> CpuUsage {
        self.cpu
    }

    /// Resident memory over total memory, at most 100 %: `MEM%`. `None` when
    /// either is unknown.
    #[must_use]
    pub const fn memory_share(&self) -> Option<Percent> {
        self.memory_share
    }
}

fn memory_share(resident: Option<MemoryBytes>, total: Option<MemoryBytes>) -> Option<Percent> {
    let share = Percent::ratio(u128::from(resident?.bytes()), u128::from(total?.bytes()))?;
    Some(share.at_most(Percent::FULL))
}

/// The CPU time that passed between two samples, which each process's CPU
/// time is a share of.
pub(super) struct CpuWindow {
    /// Every CPU's time together; `None` for a first sample, or when none
    /// passed.
    elapsed: Option<NonZeroU128>,
    /// At least one, even when the source lists no cores.
    cores: u32,
}

impl CpuWindow {
    pub(super) fn new(previous: Option<&SystemCounters>, system: &SystemSample) -> Self {
        let elapsed = previous.map(|before| total_time(&before.total_growth(system)));
        Self {
            elapsed: elapsed.and_then(NonZeroU128::new),
            cores: u32::try_from(system.cores.len()).unwrap_or(u32::MAX).max(1),
        }
    }

    /// A process's share: its CPU time since `before` over the time each CPU
    /// had, so one busy core is 100 % (htop's Irix mode), and at most every
    /// core busy. Pending without a `before` or without elapsed time.
    fn usage(&self, now: Duration, before: Option<Duration>) -> CpuUsage {
        let (Some(elapsed), Some(before)) = (self.elapsed, before) else {
            return CpuUsage::Pending;
        };
        let cores = u128::from(self.cores);
        let used = now.saturating_sub(before).as_nanos().saturating_mul(cores);
        let every_core = Percent::FULL.hundredths().saturating_mul(self.cores);
        CpuUsage::Measured(Percent::of(used, elapsed).at_most(Percent::from_hundredths(every_core)))
    }
}
