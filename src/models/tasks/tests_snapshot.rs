//! Purpose: Tests for snapshot derivation: CPU and memory shares by identity,
//! exact integer hundredths, and the baselines one sample leaves the next.

use std::time::Duration;

use super::tests_support::{counters, key, kib, measured, process, sample, system};
use super::{
    Baselines, CpuUsage, MemorySample, MemoryUsage, Percent, ProcessRow, ProcessSample, Sample,
    Snapshot,
};

mod tests_counts;
mod tests_cpu_share;
mod tests_memory_share;

/// Four cores; between `first()` and a later sample every core ran one
/// second: two in user time and two idle across all four.
const CORES: usize = 4;

fn first(processes: Vec<ProcessSample>) -> Sample {
    sample(
        system(counters(100, 0, 300), &[counters(25, 0, 75); CORES]),
        processes,
    )
}

fn one_second_later(processes: Vec<ProcessSample>) -> Sample {
    sample(
        system(counters(102, 0, 302), &[counters(25, 0, 76); CORES]),
        processes,
    )
}

fn used(raw: u32, cpu_time: Duration) -> ProcessSample {
    ProcessSample {
        cpu_time,
        ..process(raw, "worker")
    }
}

fn cpu_of(snapshot: &Snapshot) -> Vec<CpuUsage> {
    snapshot.rows().iter().map(ProcessRow::cpu).collect()
}

fn measured_share(hundredths: u32) -> CpuUsage {
    CpuUsage::Measured(Percent::from_hundredths(hundredths))
}
