//! Purpose: unit tests for sorting: every key both ways with the PID tie-break,
//! unknown amounts below known ones, and the key in force reversing.

use super::*;

/// Which end of a key's range a fixture process sits at.
#[derive(Clone, Copy)]
enum End {
    Low,
    High,
}

fn priority(kernel: i32, nice: i32) -> Priority {
    Priority::Unix {
        kernel: KernelPriority::new(kernel),
        nice: Niceness::try_from(nice).expect("a fixture nice value"),
    }
}

fn memory(virtual_kib: u64, resident_kib: u64, shared_kib: u64) -> Option<MemoryUsage> {
    Some(MemoryUsage {
        virtual_size: kib(virtual_kib),
        resident: kib(resident_kib),
        shared: Some(kib(shared_kib)),
    })
}

/// `process` moved to the `end` of `by`'s range, and to no other key's.
fn at_end(by: SortKey, end: End, mut process: ProcessSample) -> ProcessSample {
    let high = matches!(end, End::High);
    let amount = if high { 9 } else { 1 };
    let word = if high { "Beta" } else { "alpha" };
    match by {
        SortKey::Pid => {}
        SortKey::User => process.owner = Owner::Named(if high { "bob" } else { "alice" }.into()),
        SortKey::Priority => {
            process.priority = if high {
                priority(30, 10)
            } else {
                priority(10, -10)
            };
        }
        SortKey::Virtual => process.memory = memory(amount, 1, 1),
        SortKey::Resident => process.memory = memory(1, amount, 1),
        SortKey::Shared => process.memory = memory(1, 1, amount),
        SortKey::State => {
            process.state = if high {
                ProcessState::Sleeping
            } else {
                ProcessState::Running
            };
        }
        SortKey::Cpu | SortKey::CpuTime => process.cpu_time = Duration::from_millis(amount * 100),
        SortKey::Started => process.key = key(process.key.pid().get(), amount),
        SortKey::Name => process.name = word.to_owned(),
        SortKey::Command => process.command = format!("/usr/bin/{word}"),
    }
    process
}

/// PID 10 at the high end of `key`, PIDs 20 and 30 tied at the low end,
/// measured a second apart so CPU shares are known.
fn ranked(by: SortKey) -> Snapshot {
    let processes = vec![
        at_end(by, End::Low, process(30, "worker")),
        at_end(by, End::High, process(10, "worker")),
        at_end(by, End::Low, process(20, "worker")),
    ];
    let idle: Vec<ProcessSample> = processes
        .iter()
        .map(|process| ProcessSample {
            cpu_time: Duration::ZERO,
            ..process.clone()
        })
        .collect();
    measured(
        sample(system(counters(0, 0, 0), &[]), idle),
        sample(system(counters(1, 0, 0), &[]), processes),
    )
}

fn sorted(snapshot: &Snapshot, sort: Sort) -> Vec<u32> {
    let options = ViewOptions {
        sort,
        ..ViewOptions::default()
    };
    pids(snapshot, &build_view(snapshot.rows(), &options))
}

#[test]
fn every_key_sorts_both_ways_and_ties_break_by_pid_ascending() {
    let keys = SortKey::ALL.into_iter().filter(|key| *key != SortKey::Pid);
    for key in keys {
        let snapshot = ranked(key);
        let (ascending, descending) = (vec![20, 30, 10], vec![10, 20, 30]);
        let (direct, reversed) = match key.natural() {
            SortDirection::Ascending => (ascending, descending),
            SortDirection::Descending => (descending, ascending),
        };
        let forward = Sort::new(key, SortOrder::Direct);
        assert_eq!(sorted(&snapshot, forward), direct, "{key:?} direct");
        assert_eq!(
            sorted(&snapshot, forward.reversed()),
            reversed,
            "{key:?} reversed"
        );
    }
}

#[test]
fn pid_sorts_ascending_and_reverses() {
    let snapshot = ranked(SortKey::Pid);
    let direct = Sort::new(SortKey::Pid, SortOrder::Direct);
    assert_eq!(sorted(&snapshot, direct), [10, 20, 30]);
    assert_eq!(sorted(&snapshot, direct.reversed()), [30, 20, 10]);
}

#[test]
fn the_amounts_sort_heaviest_first_and_the_rest_ascending() {
    let descending = [
        SortKey::Cpu,
        SortKey::Resident,
        SortKey::Virtual,
        SortKey::Shared,
        SortKey::CpuTime,
    ];
    for key in SortKey::ALL {
        let expected = if descending.contains(&key) {
            SortDirection::Descending
        } else {
            SortDirection::Ascending
        };
        assert_eq!(key.natural(), expected, "{key:?}");
    }
}

#[test]
fn an_unknown_amount_sorts_below_every_known_one() {
    let known = ProcessSample {
        memory: memory(1, 1, 1),
        ..process(20, "known")
    };
    let snapshot = snapshot(vec![process(10, "unknown"), known]);
    let resident = Sort::new(SortKey::Resident, SortOrder::Direct);
    assert_eq!(sorted(&snapshot, resident), [20, 10]);
}

#[test]
fn choosing_the_key_in_force_reverses_it_and_another_key_starts_natural() {
    let sort = Sort::default();
    assert_eq!(sort, Sort::new(SortKey::Cpu, SortOrder::Direct));
    assert_eq!(sort.direction(), SortDirection::Descending);

    let again = sort.chosen(SortKey::Cpu);
    assert_eq!(again.order(), SortOrder::Reversed);
    assert_eq!(again.direction(), SortDirection::Ascending);

    let other = again.chosen(SortKey::Name);
    assert_eq!(
        (other.key(), other.order()),
        (SortKey::Name, SortOrder::Direct)
    );
    assert_eq!(other.direction(), SortDirection::Ascending);
}
