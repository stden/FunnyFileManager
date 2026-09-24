//! Purpose: Tests for the header meters: CPU segments that never exceed the
//! bar, grouped cores, htop's memory formula on a real host's numbers, swap
//! and the task counts.

use std::num::NonZeroUsize;
use std::time::Duration;

use super::tests_support::{counters, kib, measured, process, sample, system};
use super::{
    Baselines, CoreGroup, CpuBar, CpuCounters, CpuMeter, Load, LoadAverage, MemorySample, Meters,
    Percent, ProcessKind, ProcessSample, Snapshot, SwapSample,
};

/// The meters of a session's second sample, whose CPU counters grew by
/// `growth` since the first.
fn meters_after(growth: CpuCounters) -> Meters {
    measured(
        sample(system(CpuCounters::default(), &[]), Vec::new()),
        sample(system(growth, &[]), Vec::new()),
    )
    .meters()
    .clone()
}

fn bar_after(growth: CpuCounters) -> CpuBar {
    match meters_after(growth).cpu() {
        CpuMeter::Measured(bar) => bar,
        CpuMeter::Pending => panic!("time passed, so the bar is measured"),
    }
}

fn segments(bar: CpuBar) -> [u32; 4] {
    [bar.user(), bar.nice(), bar.kernel(), bar.virtualised()].map(Percent::hundredths)
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

/// `/proc/meminfo` of the host the design was checked on (spec 6.2).
const HOST_MEMORY_KIB: [u64; 6] = [
    131_636_496,
    25_897_000,
    6_258_456,
    52_449_252,
    6_932_888,
    225_196,
];

fn memory(total: u64, free: u64, buffers: u64, cached: u64, reclaimable: u64) -> MemorySample {
    MemorySample {
        total: kib(total),
        free: kib(free),
        available: kib(0),
        buffers: kib(buffers),
        cached: kib(cached),
        reclaimable: kib(reclaimable),
        shared: kib(0),
    }
}

fn memory_meter(sample_memory: MemorySample) -> [u64; 5] {
    let mut first = sample(system(CpuCounters::default(), &[]), Vec::new());
    first.system.memory = Some(sample_memory);
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    let meter = snapshot.meters().memory().expect("a memory meter");
    [
        meter.total(),
        meter.used(),
        meter.buffers(),
        meter.shared(),
        meter.cache(),
    ]
    .map(|amount| amount.bytes() / 1024)
}

#[test]
fn the_memory_meter_matches_the_worked_example() {
    let [total, free, buffers, cached, reclaimable, shared] = HOST_MEMORY_KIB;
    let host = MemorySample {
        shared: kib(shared),
        ..memory(total, free, buffers, cached, reclaimable)
    };
    let expected = [total, 40_098_900, buffers, shared, 59_156_944];
    assert_eq!(memory_meter(host), expected);
    // The segments add up to total − free, as htop's do.
    assert_eq!(expected[1..].iter().sum::<u64>(), total - free);
}

#[test]
fn used_memory_is_total_less_free_when_the_rest_exceeds_the_total() {
    assert_eq!(memory_meter(memory(1_000, 400, 0, 900, 0))[1], 600);
    // Free and cache exactly filling the total leave nothing used.
    assert_eq!(memory_meter(memory(1_000, 100, 0, 900, 0))[1], 0);
}

#[test]
fn memory_segments_are_cut_to_the_total_in_order() {
    assert_eq!(
        memory_meter(memory(1_000, 0, 5_000, 0, 0)),
        [1_000, 1_000, 0, 0, 0]
    );
    // More shared memory than cache: used 400 and buffers 200 leave 400.
    let over = MemorySample {
        shared: kib(500),
        ..memory(1_000, 100, 200, 300, 0)
    };
    assert_eq!(memory_meter(over), [1_000, 400, 200, 400, 0]);
}

fn swap_meter(total: u64, free: u64, cached: u64) -> [u64; 3] {
    let mut first = sample(system(CpuCounters::default(), &[]), Vec::new());
    first.system.swap = Some(SwapSample {
        total: kib(total),
        free: kib(free),
        cached: kib(cached),
    });
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    let meter = snapshot.meters().swap().expect("a swap meter");
    [meter.total(), meter.used(), meter.cache()].map(|amount| amount.bytes() / 1024)
}

#[test]
fn swap_used_leaves_out_free_and_cached_swap() {
    assert_eq!(swap_meter(2_048, 1_536, 256), [2_048, 256, 256]);
    assert_eq!(swap_meter(1_000, 900, 500), [1_000, 0, 500]);
    assert_eq!(swap_meter(1_000, 0, 5_000), [1_000, 0, 1_000]);
}

#[test]
fn the_meters_are_absent_without_memory_swap_load_or_uptime() {
    let meters = meters_after(counters(1, 1, 1));
    assert_eq!(meters.memory(), None);
    assert_eq!(meters.swap(), None);
    assert_eq!(meters.load(), None);
    assert_eq!(meters.uptime(), None);
}

#[test]
fn load_and_uptime_pass_through() {
    let mut first = sample(system(CpuCounters::default(), &[]), Vec::new());
    let load = LoadAverage::new(
        Load::from_hundredths(52),
        Load::from_hundredths(58),
        Load::from_hundredths(59),
    );
    first.system.load = Some(load);
    first.system.uptime = Some(Duration::from_secs(21_126));
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    assert_eq!(snapshot.meters().load(), Some(load));
    assert_eq!(
        snapshot.meters().uptime(),
        Some(Duration::from_secs(21_126))
    );
}

fn with_threads(raw: u32, threads: u32, kind: ProcessKind) -> ProcessSample {
    ProcessSample {
        threads,
        kind,
        ..process(raw, "task")
    }
}

#[test]
fn tasks_count_processes_extra_threads_and_kernel_threads() {
    let processes = vec![
        with_threads(1, 1, ProcessKind::User),
        with_threads(2, 4, ProcessKind::User),
        with_threads(3, 0, ProcessKind::User),
        with_threads(4, 1, ProcessKind::KernelThread),
        with_threads(5, 1, ProcessKind::KernelThread),
    ];
    let mut first = sample(system(CpuCounters::default(), &[]), processes);
    first.system.running = Some(3);
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    let tasks = snapshot.meters().tasks();
    let counted = [tasks.processes(), tasks.threads(), tasks.kernel_threads()];
    assert_eq!(counted, [3, 2, 2]);
    assert_eq!(tasks.running(), Some(3));
    assert_eq!(meters_after(counters(1, 1, 1)).tasks().running(), None);
}
