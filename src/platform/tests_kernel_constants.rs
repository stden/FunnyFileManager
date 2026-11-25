//! Purpose: Tests for the kernel constants the platform wrappers read: the
//! clock tick rate and the page size, checked against the kernel's own word.

use crate::platform::{clock_ticks_per_second, page_size};

/// `USER_HZ` is 100 on every Linux target Rust supports; asserting the exact
/// value, not `>= 1`, rules out a wrapper that answers 1.
#[cfg(target_os = "linux")]
#[test]
fn the_clock_ticks_one_hundred_times_a_second() {
    assert_eq!(clock_ticks_per_second(), 100);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn the_clock_ticks_at_more_than_once_a_second() {
    assert!(clock_ticks_per_second() > 1);
}

/// The kernel's own statement of its page size, independent of the call under
/// test, and right on arm64 kernels with 16 or 64 KiB pages.
#[cfg(target_os = "linux")]
#[test]
fn the_page_size_is_the_one_the_kernel_maps_with() {
    let smaps = std::fs::read_to_string("/proc/self/smaps").expect("smaps");
    let kib: u64 = smaps
        .lines()
        .find_map(|line| line.strip_prefix("KernelPageSize:"))
        .and_then(|rest| rest.trim().strip_suffix("kB"))
        .expect("a KernelPageSize line")
        .trim()
        .parse()
        .expect("a number of KiB");
    assert_eq!(page_size(), kib * 1024);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn the_page_size_is_a_power_of_two_of_at_least_4_kib() {
    assert!(page_size().is_power_of_two() && page_size() >= 4096);
}
