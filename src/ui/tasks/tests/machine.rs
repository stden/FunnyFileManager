//! The machine of the design's sketch the task list's screen tests share:
//! eight cores at fixed loads, its memory and swap, and the counters of two
//! samples a second apart.

use std::time::Duration;

use crate::models::{
    CoreSample, CpuCounters, Load, LoadAverage, MemoryBytes, MemorySample, SwapSample, SystemSample,
};

/// Mebibytes and gibibytes, as the sizes in the sketch are written.
const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// `tenths` tenths of a gibibyte, in bytes.
pub(super) const fn gib_tenths(tenths: u64) -> MemoryBytes {
    MemoryBytes::from_bytes(tenths * GIB / 10)
}

/// `tenths` tenths of a mebibyte, in bytes.
pub(super) const fn mib_tenths(tenths: u64) -> MemoryBytes {
    MemoryBytes::from_bytes(tenths * MIB / 10)
}

pub(super) const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// CPU counters with `user` and `idle` milliseconds.
const fn busy(user: u64, idle: u64) -> CpuCounters {
    CpuCounters {
        user: ms(user),
        nice: Duration::ZERO,
        system: Duration::ZERO,
        idle: ms(idle),
        virtualised: Duration::ZERO,
    }
}

/// Which of the sketch's two samples.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Moment {
    First,
    /// Ten CPU-seconds on: 34.2 % busy in all, each core at its load of
    /// [`CORE_LOADS`].
    Later,
}

/// The sketch's per-core loads, in tenths of a percent.
const CORE_LOADS: [u64; 8] = [120, 81, 403, 30, 95, 22, 179, 14];

/// `counters` at `moment`, `user` and `idle` milliseconds on for the later
/// sample.
fn counters_at(moment: Moment, counters: CpuCounters, user: u64, idle: u64) -> CpuCounters {
    match moment {
        Moment::First => counters,
        Moment::Later => CpuCounters {
            user: counters.user + ms(user),
            idle: counters.idle + ms(idle),
            ..counters
        },
    }
}

/// `kib` kibibytes.
fn kib(kib: u64) -> MemoryBytes {
    MemoryBytes::from_kib(kib).unwrap()
}

/// 125 G of memory, 38.2 G of it used.
fn memory() -> MemorySample {
    let total = 125 * 1024 * 1024;
    let free = total - 40_055_603;
    MemorySample {
        total: kib(total),
        free: kib(free),
        available: kib(free),
        buffers: kib(0),
        cached: kib(0),
        reclaimable: kib(0),
        shared: kib(0),
    }
}

/// The system of the sketch at `moment`: eight cores, the memory of
/// [`memory`], 2 G of swap unused.
pub(super) fn system(moment: Moment) -> SystemSample {
    let cores = CORE_LOADS
        .iter()
        .zip(0_u16..)
        .map(|(&load, index)| CoreSample {
            index,
            counters: counters_at(moment, busy(1_000, 1_000), load, 1_000 - load),
        });
    let load = |hundredths| Load::from_hundredths(hundredths);
    SystemSample {
        total: counters_at(moment, busy(10_000, 10_000), 3_420, 6_580),
        cores: cores.collect(),
        memory: Some(memory()),
        swap: Some(SwapSample {
            total: kib(2 * 1024 * 1024),
            free: kib(2 * 1024 * 1024),
            cached: kib(0),
        }),
        load: Some(LoadAverage::new(load(52), load(58), load(59))),
        uptime: Some(Duration::from_secs(5 * 3600 + 52 * 60 + 6)),
        running: Some(3),
    }
}
