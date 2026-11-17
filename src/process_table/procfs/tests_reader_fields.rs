//! Purpose: Tests for what the reader puts in a process's row: every field
//! from its `stat`, `statm`, `status` and `cmdline`, and optional files that
//! cannot be read or say nothing costing only their own field.

use std::time::Duration;

use super::tests_reader::{key, only, pid, sample, FakeProc, Process, NOBODY_KNOWS};
use crate::models::{
    KernelPriority, MemoryBytes, MemoryUsage, Niceness, Owner, Priority, ProcessKind, ProcessState,
};

#[test]
fn a_process_is_read_from_its_stat_statm_status_and_cmdline() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    let sample = sample(&mut fake.reader());
    let process = only(&sample);
    let kib = |amount| MemoryBytes::from_kib(amount).expect("a size");
    assert_eq!(process.key, key(42));
    assert_eq!(process.parent, Some(pid(1)));
    assert_eq!(process.name, "bob");
    assert_eq!(process.command, "/usr/bin/bob --flag");
    assert_eq!(process.owner, Owner::Uid(NOBODY_KNOWS));
    assert_eq!(process.state, ProcessState::Sleeping);
    let nice = Niceness::try_from(0).expect("a nice value");
    let kernel = KernelPriority::new(20);
    assert_eq!(process.priority, Priority::Unix { kernel, nice });
    assert_eq!(process.threads, 2);
    assert_eq!(process.cpu_time, Duration::from_secs(3));
    let memory = MemoryUsage {
        virtual_size: kib(8000),
        resident: kib(400),
        shared: Some(kib(200)),
    };
    assert_eq!(process.memory, Some(memory));
    assert_eq!(process.running_for, Some(Duration::from_millis(996_300)));
    assert_eq!(process.kind, ProcessKind::User);
}

#[test]
fn optional_files_that_cannot_be_read_cost_only_their_field() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    for file in ["42/statm", "42/status", "42/cmdline"] {
        fake.unreadable(file);
    }
    let sample = sample(&mut fake.reader());
    let process = only(&sample);
    assert_eq!(process.memory, None);
    assert_eq!(process.owner, Owner::Unknown);
    assert_eq!(process.command, "[bob]");
    assert_eq!(sample.unreadable, 0);
}

#[test]
fn a_status_without_an_effective_uid_leaves_the_owner_unknown() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.write("42/status", "Name:\tbob\nUid:\t1000\n");
    assert_eq!(only(&sample(&mut fake.reader())).owner, Owner::Unknown);
}
