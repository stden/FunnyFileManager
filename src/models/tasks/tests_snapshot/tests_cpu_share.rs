//! Purpose: unit tests for CPU shares: exact hundredths per CPU, pending and zero
//! cases, the cap at every core busy, and the baselines one sample leaves the next.

use super::*;

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
