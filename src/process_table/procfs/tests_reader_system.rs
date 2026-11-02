//! Purpose: Tests for the system-wide side of the reader: the counters and
//! meters of the system files, optional files that only cost their meter,
//! the failures that fail a whole sample, and one smoke test against the
//! real `/proc`.

use std::io::ErrorKind;
use std::time::Duration;

use super::tests_reader::{hertz, only, page, sample, FakeProc, Process};
use super::ProcFs;
use crate::models::{CpuCounters, MemoryBytes, ProcessReader, SampleError, SystemPart};

#[test]
fn the_system_files_give_the_counters_and_the_meters() {
    let fake = FakeProc::new();
    let system = sample(&mut fake.reader()).system;
    let at = |ticks: u64| Duration::from_millis(ticks * 10);
    let counters = CpuCounters {
        user: at(91),
        nice: at(10),
        system: at(43),
        idle: at(450),
        virtualised: at(27),
    };
    assert_eq!(system.total, counters);
    assert_eq!(system.cores.len(), 1);
    assert_eq!(system.running, Some(3));
    assert_eq!(system.uptime, Some(Duration::from_millis(1_000_500)));
    assert!(system.load.is_some());
    let memory = system.memory.expect("a memory sample");
    assert_eq!(memory.total, MemoryBytes::from_kib(1000).expect("a size"));
    assert_eq!(
        memory.available,
        MemoryBytes::from_kib(600).expect("a size")
    );
    let swap = system.swap.expect("a swap sample");
    assert_eq!(swap.free, MemoryBytes::from_kib(150).expect("a size"));
}

#[test]
fn optional_system_files_that_are_missing_cost_only_their_values() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    for file in ["meminfo", "loadavg", "uptime"] {
        fake.remove(file);
    }
    let sample = sample(&mut fake.reader());
    let system = &sample.system;
    assert_eq!(
        (system.memory, system.swap, system.load, system.uptime),
        (None, None, None, None)
    );
    assert_eq!(only(&sample).running_for, None);
}

#[test]
fn a_missing_process_table_is_an_unreadable_process_list() {
    let fake = FakeProc::new();
    let mut reader = ProcFs::new(fake.root().join("absent"), hertz(), page());
    let expected = SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: ErrorKind::NotFound,
    };
    assert_eq!(reader.sample().err(), Some(expected));
}

#[test]
fn a_missing_or_malformed_proc_stat_fails_the_sample() {
    let fake = FakeProc::new();
    fake.add(&Process::new(42, "bob"));
    fake.write("stat", "cpu 1 2\n");
    let malformed = SampleError::Malformed(SystemPart::CpuTimes);
    assert_eq!(fake.reader().sample().err(), Some(malformed));
    fake.remove("stat");
    let missing = SampleError::Unreadable {
        part: SystemPart::CpuTimes,
        kind: ErrorKind::NotFound,
    };
    assert_eq!(fake.reader().sample().err(), Some(missing));
}

/// The one test of the real `/proc`: it finds this process, started when
/// its own `stat` says.
#[cfg(target_os = "linux")]
#[test]
fn the_real_proc_lists_this_process_with_its_start_time() {
    use std::fs;

    use super::tests_reader::pid;
    use crate::models::{ProcessKey, Start, StartTime};

    let own = pid(std::process::id());
    let own_stat = fs::read_to_string("/proc/self/stat").expect("own stat");
    let (_, fields) = own_stat.rsplit_once(')').expect("a closing parenthesis");
    let start: u64 = fields
        .split_ascii_whitespace()
        .nth(22 - 3)
        .expect("a start field")
        .parse()
        .expect("a numeric start");
    let mut reader = ProcFs::system().unwrap_or_else(|error| panic!("a reader: {error}"));
    let sample = sample(&mut reader);
    let expected = ProcessKey::new(own, Start::Known(StartTime::new(start)));
    assert!(sample
        .processes
        .iter()
        .any(|process| process.key == expected));
    assert!(sample.system.memory.is_some() && sample.system.uptime.is_some());
}
