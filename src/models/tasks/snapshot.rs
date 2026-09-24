//! Purpose: A snapshot — one sample turned into rows with their CPU and memory
//! shares, plus the header meters — measured against the baselines the
//! previous sample left.
//!
//! Derivation lives here, once, for every platform: a source only reports
//! cumulative counters. Integer maths only; see `units.rs` for why.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::num::NonZeroU128;
use std::time::Duration;

use super::meters::{total_time, SystemCounters};
use super::{
    CpuUsage, MemoryBytes, Meters, Niceness, Percent, Priority, ProcessKey, ProcessSample, Sample,
    SystemSample, TaskCounts,
};

/// What a sample leaves for the next one to be measured from: the system's CPU
/// counters and each process's CPU time, by identity. A lookup table, never
/// drawn.
#[derive(Clone, Default)]
pub struct Baselines {
    system: Option<SystemCounters>,
    cpu_time: HashMap<ProcessKey, Duration>,
}

/// The process table as last derived.
///
/// No `Debug`: its rows hold command lines, which may hold secrets.
#[derive(Clone)]
pub struct Snapshot {
    /// In the source's order, one row per [`ProcessKey`].
    rows: Vec<ProcessRow>,
    meters: Meters,
    unreadable: u32,
}

impl Snapshot {
    /// The snapshot of `sample` measured against `previous`, and the
    /// baselines the next sample is to be measured against.
    ///
    /// A key the source reported twice keeps its first row, so rows are unique
    /// by identity whatever the source does.
    #[must_use]
    pub fn derive(previous: &Baselines, sample: Sample) -> (Self, Baselines) {
        let Sample {
            system,
            processes,
            unreadable,
        } = sample;
        let window = CpuWindow::new(previous.system.as_ref(), &system);
        let memory_total = system.memory.map(|memory| memory.total);
        let mut cpu_time = HashMap::with_capacity(processes.len());
        let rows: Vec<ProcessRow> = processes
            .into_iter()
            .filter(|process| is_first_of_its_key(&mut cpu_time, process))
            .map(|process| {
                let before = previous.cpu_time.get(&process.key).copied();
                ProcessRow::derive(process, before, &window, memory_total)
            })
            .collect();
        let tasks = TaskCounts::of(rows.iter().map(ProcessRow::sample), system.running);
        let meters = Meters::derive(previous.system.as_ref(), &system, tasks);
        let baselines = Baselines {
            system: Some(SystemCounters::of(&system)),
            cpu_time,
        };
        let snapshot = Self {
            rows,
            meters,
            unreadable,
        };
        (snapshot, baselines)
    }

    #[must_use]
    pub fn rows(&self) -> &[ProcessRow] {
        &self.rows
    }

    #[must_use]
    pub const fn meters(&self) -> &Meters {
        &self.meters
    }

    /// How many cores had counters in the sample: what the header sizes its
    /// core bars by.
    #[must_use]
    pub fn cores(&self) -> usize {
        self.meters.cores().count()
    }

    /// Processes the source skipped because their files could not be read.
    #[must_use]
    pub const fn unreadable(&self) -> u32 {
        self.unreadable
    }

    /// Records that the process `key` now runs at nice `nice`, as a renice
    /// the kernel accepted says; the next sample reads it back.
    pub(super) fn record_nice(&mut self, key: ProcessKey, nice: Niceness) {
        let row = self.rows.iter_mut().find(|row| row.sample.key == key);
        if let Some(Priority::Unix { nice: recorded, .. }) = row.map(|row| &mut row.sample.priority)
        {
            *recorded = nice;
        }
    }
}

/// Records `process`'s CPU time under its key, and answers whether the key was
/// new; a repeated key is not recorded again.
fn is_first_of_its_key(seen: &mut HashMap<ProcessKey, Duration>, process: &ProcessSample) -> bool {
    match seen.entry(process.key) {
        Entry::Vacant(slot) => {
            slot.insert(process.cpu_time);
            true
        }
        Entry::Occupied(_) => false,
    }
}

/// One process as sampled, with the shares derived for it.
///
/// No `Debug`, for the same reason as [`Snapshot`].
#[derive(Clone)]
pub struct ProcessRow {
    sample: ProcessSample,
    cpu: CpuUsage,
    memory_share: Option<Percent>,
}

impl ProcessRow {
    fn derive(
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
struct CpuWindow {
    /// Every CPU's time together; `None` for a first sample, or when none
    /// passed.
    elapsed: Option<NonZeroU128>,
    /// At least one, even when the source lists no cores.
    cores: u32,
}

impl CpuWindow {
    fn new(previous: Option<&SystemCounters>, system: &SystemSample) -> Self {
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
