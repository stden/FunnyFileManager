//! Purpose: Tests for snapshot derivation: CPU and memory shares by identity,
//! exact integer hundredths, and the baselines one sample leaves the next.

use std::time::Duration;

use super::tests_support::{counters, key, kib, measured, process, sample, system};
use super::{
    Baselines, CpuUsage, MemorySample, MemoryUsage, Percent, ProcessRow, ProcessSample, Sample,
    Snapshot,
};

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

#[test]
fn the_first_sample_measures_no_cpu_share_yet() {
    let (snapshot, _) =
        Snapshot::derive(&Baselines::default(), first(vec![used(7, Duration::ZERO)]));
    assert_eq!(cpu_of(&snapshot), [CpuUsage::Pending]);
}

#[test]
fn the_second_sample_measures_exact_hundredths_per_cpu() {
    let before = vec![
        used(7, Duration::from_secs(10)),
        used(8, Duration::from_secs(20)),
    ];
    let after = vec![
        used(7, Duration::from_millis(10_500)),
        used(8, Duration::from_millis(21_500)),
    ];
    let snapshot = measured(first(before), one_second_later(after));
    // Half a second of one CPU is 50 %; a second and a half across cores is 150 %.
    assert_eq!(
        cpu_of(&snapshot),
        [measured_share(5_000), measured_share(15_000)]
    );
}

#[test]
fn a_reused_pid_with_a_new_start_is_pending_not_huge() {
    let before = vec![used(7, Duration::from_secs(500))];
    let newcomer = ProcessSample {
        key: key(7, 9_999),
        ..used(7, Duration::from_secs(1))
    };
    let snapshot = measured(first(before), one_second_later(vec![newcomer]));
    assert_eq!(cpu_of(&snapshot), [CpuUsage::Pending]);
}

#[test]
fn cpu_time_that_goes_backwards_measures_zero() {
    let before = vec![used(7, Duration::from_secs(10))];
    let after = vec![used(7, Duration::from_secs(9))];
    let snapshot = measured(first(before), one_second_later(after));
    assert_eq!(cpu_of(&snapshot), [measured_share(0)]);
}

#[test]
fn with_no_time_between_samples_every_share_is_pending() {
    let before = vec![used(7, Duration::from_secs(10))];
    let after = vec![used(7, Duration::from_secs(11))];
    let snapshot = measured(first(before), first(after));
    assert_eq!(cpu_of(&snapshot), [CpuUsage::Pending]);
}

#[test]
fn system_counters_that_go_backwards_count_no_time() {
    let before = vec![used(7, Duration::from_secs(10))];
    let after = sample(
        system(counters(1, 0, 1), &[]),
        vec![used(7, Duration::from_secs(11))],
    );
    let snapshot = measured(first(before), after);
    assert_eq!(cpu_of(&snapshot), [CpuUsage::Pending]);
}

#[test]
fn a_share_is_at_most_every_core_busy() {
    let before = vec![used(7, Duration::ZERO)];
    let after = vec![used(7, Duration::from_secs(60))];
    let snapshot = measured(first(before), one_second_later(after));
    assert_eq!(cpu_of(&snapshot), [measured_share(40_000)]);
}

#[test]
fn a_source_without_cores_measures_shares_of_one_cpu() {
    let at = |seconds, cpu_time| {
        sample(
            system(counters(seconds, 0, seconds), &[]),
            vec![used(7, cpu_time)],
        )
    };
    let snapshot = measured(
        at(10, Duration::from_secs(1)),
        at(11, Duration::from_millis(1_500)),
    );
    // Two seconds elapsed on one CPU; half a second used is 25 %.
    assert_eq!(cpu_of(&snapshot), [measured_share(2_500)]);
    assert_eq!(snapshot.cores(), 0);
}

#[test]
fn the_next_sample_is_measured_from_the_latest_not_the_first() {
    let (_, baselines) =
        Snapshot::derive(&Baselines::default(), first(vec![used(7, Duration::ZERO)]));
    let (_, baselines) = Snapshot::derive(
        &baselines,
        one_second_later(vec![used(7, Duration::from_secs(1))]),
    );
    let third = sample(
        system(counters(104, 0, 304), &[counters(25, 0, 77); CORES]),
        vec![used(7, Duration::from_millis(1_250))],
    );
    let (snapshot, _) = Snapshot::derive(&baselines, third);
    assert_eq!(cpu_of(&snapshot), [measured_share(2_500)]);
}

#[test]
fn a_key_reported_twice_keeps_its_first_row() {
    let twice = vec![
        used(7, Duration::from_secs(1)),
        ProcessSample {
            name: "impostor".to_owned(),
            ..used(7, Duration::from_secs(90))
        },
    ];
    let (_, baselines) =
        Snapshot::derive(&Baselines::default(), first(vec![used(7, Duration::ZERO)]));
    let (snapshot, baselines) = Snapshot::derive(&baselines, one_second_later(twice));
    let names: Vec<&str> = snapshot
        .rows()
        .iter()
        .map(|row| row.sample().name.as_str())
        .collect();
    assert_eq!(names, ["worker"]);
    assert_eq!(cpu_of(&snapshot), [measured_share(10_000)]);
    // The next sample is measured from the kept row's CPU time, not the other.
    let third = sample(
        system(counters(104, 0, 304), &[counters(25, 0, 77); CORES]),
        vec![used(7, Duration::from_secs(2))],
    );
    let (snapshot, _) = Snapshot::derive(&baselines, third);
    assert_eq!(cpu_of(&snapshot), [measured_share(10_000)]);
}

fn resident(raw: u32, kib_resident: u64) -> ProcessSample {
    ProcessSample {
        memory: Some(MemoryUsage {
            virtual_size: kib(kib_resident * 2),
            resident: kib(kib_resident),
            shared: None,
        }),
        ..process(raw, "cache")
    }
}

fn with_memory_total(total_kib: u64, processes: Vec<ProcessSample>) -> Sample {
    let mut sample = first(processes);
    sample.system.memory = Some(MemorySample {
        total: kib(total_kib),
        free: kib(0),
        available: kib(0),
        buffers: kib(0),
        cached: kib(0),
        reclaimable: kib(0),
        shared: kib(0),
    });
    sample
}

#[test]
fn memory_share_is_resident_over_total_and_at_most_full() {
    let processes = vec![
        resident(1, 250),
        resident(2, 4_000),
        process(3, "unmeasured"),
    ];
    let (snapshot, _) =
        Snapshot::derive(&Baselines::default(), with_memory_total(1_000, processes));
    let shares: Vec<Option<Percent>> = snapshot
        .rows()
        .iter()
        .map(ProcessRow::memory_share)
        .collect();
    let expected = [
        Some(Percent::from_hundredths(2_500)),
        Some(Percent::FULL),
        None,
    ];
    assert_eq!(shares, expected);
}

#[test]
fn memory_share_is_unknown_without_a_nonzero_total() {
    let without = first(vec![resident(1, 250)]);
    let empty = with_memory_total(0, vec![resident(1, 250)]);
    for sample in [without, empty] {
        let (snapshot, _) = Snapshot::derive(&Baselines::default(), sample);
        assert_eq!(snapshot.rows()[0].memory_share(), None);
    }
}

#[test]
fn the_unreadable_count_and_the_cores_pass_through() {
    let mut sample = first(Vec::new());
    sample.unreadable = 3;
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), sample);
    assert_eq!(snapshot.unreadable(), 3);
    assert_eq!(snapshot.cores(), CORES);
    assert!(snapshot.rows().is_empty());
}
