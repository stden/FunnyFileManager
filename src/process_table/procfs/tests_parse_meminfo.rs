//! Purpose: Tests for the `/proc/meminfo` parser: keys in any order, the
//! available amount estimated without `MemAvailable`, no meter without a
//! total, and sizes too large for bytes left out.

use super::parse::memory_info;
use crate::models::{MemoryBytes, MemorySample, SwapSample};

/// This host's `/proc/meminfo` from the design, keys shuffled.
const MEMINFO: &str = "\
Shmem:            225196 kB
MemFree:        25897000 kB
MemTotal:       131636496 kB
HugePages_Total:       0
Cached:         52449252 kB
MemAvailable:   89695600 kB
SReclaimable:    6932888 kB
Buffers:         6258456 kB
SwapCached:          512 kB
SwapTotal:       2097148 kB
SwapFree:        2096636 kB
";

fn kib(amount: u64) -> MemoryBytes {
    MemoryBytes::from_kib(amount).expect("a fixture size")
}

#[test]
fn meminfo_reads_its_keys_in_any_order() {
    let info = memory_info(MEMINFO);
    let memory = MemorySample {
        total: kib(131_636_496),
        free: kib(25_897_000),
        available: kib(89_695_600),
        buffers: kib(6_258_456),
        cached: kib(52_449_252),
        reclaimable: kib(6_932_888),
        shared: kib(225_196),
    };
    assert_eq!(info.memory, Some(memory));
    let swap = SwapSample {
        total: kib(2_097_148),
        free: kib(2_096_636),
        cached: kib(512),
    };
    assert_eq!(info.swap, Some(swap));
}

#[test]
fn without_mem_available_free_buffers_and_cache_are_available() {
    let old = MEMINFO.replace("MemAvailable:   89695600 kB\n", "");
    let available = memory_info(&old).memory.map(|memory| memory.available);
    assert_eq!(available, Some(kib(25_897_000 + 6_258_456 + 52_449_252)));
}

#[test]
fn without_a_total_there_is_no_meter() {
    let no_total = MEMINFO.replace("MemTotal:       131636496 kB\n", "");
    let info = memory_info(&no_total);
    assert_eq!(info.memory, None);
    assert!(info.swap.is_some());
    let no_swap = memory_info("MemTotal: 8 kB\n");
    assert_eq!(no_swap.swap, None);
    assert_eq!(no_swap.memory.map(|memory| memory.free), Some(kib(0)));
}

#[test]
fn a_size_too_large_for_bytes_is_left_out() {
    let huge = "MemTotal: 18446744073709551615 kB\nMemTotal: 8 kB\n";
    assert_eq!(
        memory_info(huge).memory.map(|memory| memory.total),
        Some(kib(8))
    );
    assert_eq!(memory_info("garbage\n:\nMemTotal:\n").memory, None);
}
