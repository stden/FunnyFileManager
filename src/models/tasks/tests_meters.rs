//! Purpose: Tests for the header meters: CPU segments that never exceed the
//! bar, grouped cores, htop's memory formula on a real host's numbers, swap
//! and the task counts.

use std::num::NonZeroUsize;
use std::time::Duration;

use super::tests_support::{counters, kib, measured, process, sample, system};
use super::{
    Baselines, CoreGroup, CpuBar, CpuCounters, CpuMeter, Load, LoadAverage, MemorySample, Meters,
    Percent, ProcessKind, ProcessSample, Snapshot, SwapSample,
};

mod tests_counts;
mod tests_cpu;
mod tests_memory;

/// The meters of a session's second sample, whose CPU counters grew by
/// `growth` since the first.
fn meters_after(growth: CpuCounters) -> Meters {
    measured(
        sample(system(CpuCounters::default(), &[]), Vec::new()),
        sample(system(growth, &[]), Vec::new()),
    )
    .meters()
    .clone()
}

fn segments(bar: CpuBar) -> [u32; 4] {
    [bar.user(), bar.nice(), bar.kernel(), bar.virtualised()].map(Percent::hundredths)
}

fn memory(total: u64, free: u64, buffers: u64, cached: u64, reclaimable: u64) -> MemorySample {
    MemorySample {
        total: kib(total),
        free: kib(free),
        available: kib(0),
        buffers: kib(buffers),
        cached: kib(cached),
        reclaimable: kib(reclaimable),
        shared: kib(0),
    }
}
