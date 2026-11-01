//! Purpose: Builders for samples, snapshots and sampled lists, shared by the
//! task-list tests: fixture processes owned by `bob`, CPU counters in whole
//! seconds, memory in KiB. The sample builders are `test_support::processes`'s,
//! which the tests outside the crate use too; these take a bare PID.

use std::time::Instant;

use super::{
    Baselines, MemoryBytes, Pid, ProcessKey, ProcessSample, Sample, Snapshot, Start, StartTime,
    TaskList, TaskSession,
};
use crate::prompt::Privilege;
pub use crate::test_support::processes::{counters, one_core, sample, system};

pub fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a fixture PID is in range")
}

/// The key of process `raw`, started at tick `start`.
pub fn key(raw: u32, start: u64) -> ProcessKey {
    ProcessKey::new(pid(raw), Start::Known(StartTime::new(start)))
}

/// A sleeping one-thread user process of bob's, started at tick `raw`, with no
/// CPU time used and no memory figures. Tests change fields with struct
/// update syntax.
pub fn process(raw: u32, name: &str) -> ProcessSample {
    crate::test_support::processes::process(pid(raw), name)
}

pub fn kib(amount: u64) -> MemoryBytes {
    MemoryBytes::from_kib(amount).expect("a fixture size fits")
}

/// The snapshot of `second`, measured against `first`.
pub fn measured(first: Sample, second: Sample) -> Snapshot {
    let (_, baselines) = Snapshot::derive(&Baselines::default(), first);
    Snapshot::derive(&baselines, second).0
}

/// The snapshot of a session's first sample of `processes` on one core:
/// every CPU share still pending.
pub fn snapshot(processes: Vec<ProcessSample>) -> Snapshot {
    Snapshot::derive(&Baselines::default(), one_core(processes)).0
}

/// FFM's own PID in the fixtures.
pub const OWN: u32 = 900;

/// A list of session 1, run by [`OWN`], that asked for and applied one sample
/// of `processes`: the first snapshot, sorted by PID since every CPU share is
/// still pending.
pub fn listing(processes: Vec<ProcessSample>) -> TaskList {
    let mut list = TaskList::new(TaskSession::new(1), pid(OWN), Privilege::User);
    resample(&mut list, processes);
    list
}

/// `list` asks for and applies the next sample, of `processes`.
pub fn resample(list: &mut TaskList, processes: Vec<ProcessSample>) {
    let now = Instant::now();
    list.take_request(now);
    list.apply_sample(Ok(one_core(processes)), now);
}

/// The PIDs of `list`'s shown rows, in order.
pub fn shown(list: &TaskList) -> Vec<u32> {
    list.view()
        .iter()
        .map(|row| row.key().pid().get())
        .collect()
}

/// `process` as a child of `parent`.
pub fn child_of(parent: u32, process: ProcessSample) -> ProcessSample {
    ProcessSample {
        parent: Some(pid(parent)),
        ..process
    }
}
