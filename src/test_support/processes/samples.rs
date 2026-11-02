//! Purpose: Builders of the samples the fake process table serves: a
//! sleeping process of bob's, CPU counters, a system and a whole sample.

use std::time::Duration;

use crate::models::tasks::NetworkSample;
use crate::models::{
    CoreSample, CpuCounters, KernelPriority, Niceness, Owner, Pid, Priority, ProcessKey,
    ProcessKind, ProcessSample, ProcessState, Sample, Start, StartTime, SystemSample,
};

/// A sleeping one-thread user process of bob's, `/usr/bin/<name>`, started
/// at tick `pid`, with no CPU time used and no memory figures. Tests change
/// fields with struct update syntax.
#[must_use]
pub fn process(pid: Pid, name: &str) -> ProcessSample {
    ProcessSample {
        key: ProcessKey::new(pid, Start::Known(StartTime::new(u64::from(pid.get())))),
        parent: None,
        name: name.to_owned(),
        command: format!("/usr/bin/{name}"),
        owner: Owner::Named("bob".to_owned()),
        state: ProcessState::Sleeping,
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: Niceness::NORMAL,
        },
        threads: 1,
        cpu_time: Duration::ZERO,
        memory: None,
        running_for: None,
        kind: ProcessKind::User,
    }
}

/// CPU counters in whole seconds; nice and virtualised time are zero.
#[must_use]
pub const fn counters(user: u64, system: u64, idle: u64) -> CpuCounters {
    CpuCounters {
        user: Duration::from_secs(user),
        nice: Duration::ZERO,
        system: Duration::from_secs(system),
        idle: Duration::from_secs(idle),
        virtualised: Duration::ZERO,
    }
}

/// A system whose `cores` are numbered from 0, with no memory, swap, load,
/// uptime or runnable count.
#[must_use]
pub fn system(total: CpuCounters, cores: &[CpuCounters]) -> SystemSample {
    let cores = cores
        .iter()
        .zip(0_u16..)
        .map(|(counters, index)| CoreSample {
            index,
            counters: *counters,
        });
    SystemSample {
        total,
        cores: cores.collect(),
        memory: None,
        swap: None,
        load: None,
        uptime: None,
        running: None,
    }
}

/// `processes` on `system`, every one of them readable.
#[must_use]
pub const fn sample(system: SystemSample, processes: Vec<ProcessSample>) -> Sample {
    Sample {
        system,
        processes,
        unreadable: 0,
        network: NetworkSample::empty(),
    }
}

/// A sample of `processes` on one core that has run one second of each
/// kind of time.
#[must_use]
pub fn one_core(processes: Vec<ProcessSample>) -> Sample {
    sample(system(counters(1, 1, 1), &[counters(1, 1, 1)]), processes)
}
