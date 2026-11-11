//! Purpose: A snapshot — one sample turned into rows with their CPU and memory
//! shares, network adapter rates, plus the header meters — measured against
//! the baselines the previous sample left.
//!
//! Derivation lives here, once, for every platform: a source only reports
//! cumulative counters. Integer maths only; see `units.rs` for why.

mod row;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;

use super::meters::SystemCounters;
use super::network::{derive_network, NetBaselines};
use super::{
    AdapterRow, Meters, Niceness, Priority, PriorityChange, ProcessKey, ProcessSample, Sample,
    SystemSample, TaskCounts,
};

pub use row::ProcessRow;

use row::CpuWindow;

/// What a sample leaves for the next one to be measured from: the system's CPU
/// counters, each process's CPU time, and adapter byte counters.
#[derive(Clone, Default)]
pub struct Baselines {
    system: Option<SystemCounters>,
    cpu_time: HashMap<ProcessKey, Duration>,
    net: NetBaselines,
}

/// The process and network tables as last derived.
///
/// No `Debug`: its rows hold command lines, which may hold secrets.
#[derive(Clone)]
pub struct Snapshot {
    /// In the source's order, one row per [`ProcessKey`].
    rows: Vec<ProcessRow>,
    adapters: Vec<AdapterRow>,
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
            network,
        } = sample;
        let (rows, cpu_time) = derive_rows(previous, &system, processes);
        let (adapters, net_meter, net) = derive_network(&previous.net, network);
        let tasks = TaskCounts::of(rows.iter().map(ProcessRow::sample), system.running);
        let meters = Meters::derive(previous.system.as_ref(), &system, net_meter, tasks);
        let baselines = Baselines {
            system: Some(SystemCounters::of(&system)),
            cpu_time,
            net,
        };
        let snapshot = Self {
            rows,
            adapters,
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
    pub fn adapters(&self) -> &[AdapterRow] {
        &self.adapters
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

    /// Records that the process `key` now runs at `change` until the next
    /// sample reads it back.
    pub(super) fn record_priority(&mut self, key: ProcessKey, change: PriorityChange) {
        let Some(row) = self.rows.iter_mut().find(|row| row.sample.key == key) else {
            return;
        };
        match (&mut row.sample.priority, change) {
            (Priority::Unix { nice: recorded, .. }, PriorityChange::Nice(nice)) => {
                *recorded = nice;
            }
            (Priority::Windows { class }, PriorityChange::Class(settable)) => {
                *class = settable.as_class();
            }
            _ => {}
        }
    }

    pub(super) fn record_nice(&mut self, key: ProcessKey, nice: Niceness) {
        self.record_priority(key, PriorityChange::Nice(nice));
    }
}

fn derive_rows(
    previous: &Baselines,
    system: &SystemSample,
    processes: Vec<ProcessSample>,
) -> (Vec<ProcessRow>, HashMap<ProcessKey, Duration>) {
    let window = CpuWindow::new(previous.system.as_ref(), system);
    let memory_total = system.memory.map(|memory| memory.total);
    let mut cpu_time = HashMap::with_capacity(processes.len());
    let rows = processes
        .into_iter()
        .filter(|process| is_first_of_its_key(&mut cpu_time, process))
        .map(|process| {
            let before = previous.cpu_time.get(&process.key).copied();
            ProcessRow::derive(process, before, &window, memory_total)
        })
        .collect();
    (rows, cpu_time)
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
