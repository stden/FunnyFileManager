//! Purpose: A snapshot — one sample turned into rows with their CPU and memory
//! shares, plus the header meters — measured against the baselines the
//! previous sample left.
//!
//! Derivation lives here, once, for every platform: a source only reports
//! cumulative counters. Integer maths only; see `units.rs` for why.

mod row;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;

use super::meters::SystemCounters;
use super::{Meters, Niceness, Priority, ProcessKey, ProcessSample, Sample, TaskCounts};

pub use row::ProcessRow;

use row::CpuWindow;

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
