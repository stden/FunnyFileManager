//! Purpose: unit tests for the memory and swap meters: htop's memory formula on a
//! real host's numbers, segments cut to the total, and used swap.

use super::*;

/// `/proc/meminfo` of the host the design was checked on (spec 6.2).
const HOST_MEMORY_KIB: [u64; 6] = [
    131_636_496,
    25_897_000,
    6_258_456,
    52_449_252,
    6_932_888,
    225_196,
];

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
