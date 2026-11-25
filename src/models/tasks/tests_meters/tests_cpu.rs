//! Purpose: unit tests for the CPU bars: segments that divide the time that passed
//! and never exceed the bar, the pending meter, and cores grouped to a bar.

use super::*;

fn bar_after(growth: CpuCounters) -> CpuBar {
    match meters_after(growth).cpu() {
        CpuMeter::Measured(bar) => bar,
        CpuMeter::Pending => panic!("time passed, so the bar is measured"),
    }
}

fn seconds(user: u64, nice: u64, system: u64, idle: u64, virtualised: u64) -> CpuCounters {
    CpuCounters {
        user: Duration::from_secs(user),
        nice: Duration::from_secs(nice),
        system: Duration::from_secs(system),
        idle: Duration::from_secs(idle),
        virtualised: Duration::from_secs(virtualised),
    }
}

#[test]
fn a_cpu_bar_divides_the_time_that_passed() {
    let bar = bar_after(seconds(2, 1, 1, 4, 0));
    assert_eq!(segments(bar), [2_500, 1_250, 1_250, 0]);
    assert_eq!(bar.busy(), Percent::from_hundredths(5_000));
}

/// Sixths rounded one at a time add up to 10 001 hundredths; ending each
/// segment at a rounded running total cannot.
#[test]
fn rounded_segments_never_add_up_to_more_than_the_bar() {
    let bar = bar_after(seconds(1, 1, 1, 0, 3));
    assert_eq!(segments(bar), [1_667, 1_666, 1_667, 5_000]);
    assert_eq!(bar.busy(), Percent::FULL);
}

#[test]
fn busy_is_everything_but_idle_for_any_mix_of_parts() {
    for parts in 0..4_u64.pow(5) {
        let digit = |place: u32| (parts / 4_u64.pow(place)) % 4;
        let growth = seconds(digit(0), digit(1), digit(2), digit(3) + 1, digit(4));
        let bar = bar_after(growth);
        let sum: u32 = segments(bar).iter().sum();
        let busy_time = u128::from(digit(0) + digit(1) + digit(2) + digit(4));
        let whole = busy_time + u128::from(digit(3) + 1);
        assert_eq!(
            Some(bar.busy()),
            Percent::ratio(busy_time, whole),
            "{parts}"
        );
        assert_eq!(sum, bar.busy().hundredths(), "{parts}");
        assert!(bar.busy() <= Percent::FULL, "{parts}");
    }
}

#[test]
fn a_first_sample_or_no_time_passing_leaves_the_cpu_meter_pending() {
    let (first, _) = Snapshot::derive(
        &Baselines::default(),
        sample(system(counters(5, 5, 5), &[counters(5, 5, 5)]), Vec::new()),
    );
    assert_eq!(first.meters().cpu(), CpuMeter::Pending);
    assert_eq!(
        meters_after(CpuCounters::default()).cpu(),
        CpuMeter::Pending
    );
}

/// Six cores; core `n` was busy for `n` of the five seconds.
fn six_cores() -> Meters {
    let cores: Vec<CpuCounters> = (0..6).map(|busy| counters(busy, 0, 5 - busy)).collect();
    measured(
        sample(
            system(CpuCounters::default(), &[CpuCounters::default(); 6]),
            Vec::new(),
        ),
        sample(system(counters(15, 0, 15), &cores), Vec::new()),
    )
    .meters()
    .clone()
}

/// Each bar as (first core, last core, busy hundredths); `None` while pending.
fn bars(meters: &Meters, per_bar: usize) -> Vec<(u16, u16, Option<u32>)> {
    let per_bar = NonZeroUsize::new(per_bar).expect("at least one core per bar");
    let busy = |meter| match meter {
        CpuMeter::Measured(bar) => Some(CpuBar::busy(bar).hundredths()),
        CpuMeter::Pending => None,
    };
    let groups = meters.cores().grouped(per_bar);
    let bar = |group: &CoreGroup| (group.first_core(), group.last_core(), busy(group.meter()));
    groups.iter().map(bar).collect()
}

#[test]
fn cores_are_grouped_one_two_or_six_to_a_bar() {
    let meters = six_cores();
    assert_eq!(meters.cores().count(), 6);
    let one: Vec<(u16, u16, Option<u32>)> = (0..6)
        .map(|core| (core, core, Some(u32::from(core) * 2_000)))
        .collect();
    assert_eq!(bars(&meters, 1), one);
    // A group is one CPU: 0+1 busy of 10 seconds, 2+3 of 10, 4+5 of 10.
    let two = [
        (0, 1, Some(1_000)),
        (2, 3, Some(5_000)),
        (4, 5, Some(9_000)),
    ];
    assert_eq!(bars(&meters, 2), two);
    assert_eq!(bars(&meters, 6), [(0, 5, Some(5_000))]);
}

#[test]
fn a_core_seen_once_is_pending_and_a_group_measures_the_rest() {
    let mut second = system(counters(8, 0, 8), &[counters(2, 0, 2), counters(0, 0, 0)]);
    second.cores[1].index = 3;
    let meters = measured(
        sample(
            system(CpuCounters::default(), &[CpuCounters::default()]),
            Vec::new(),
        ),
        sample(second, Vec::new()),
    )
    .meters()
    .clone();
    assert_eq!(bars(&meters, 1), [(0, 0, Some(5_000)), (3, 3, None)]);
    assert_eq!(bars(&meters, 2), [(0, 3, Some(5_000))]);
}
