//! Purpose: Tests for the task list's units: exact integer shares, checked
//! memory conversions and load in hundredths.

use std::num::NonZeroU128;

use super::{CpuUsage, Load, LoadAverage, MemoryBytes, Percent};

#[test]
fn a_share_of_nothing_is_none() {
    assert_eq!(Percent::ratio(0, 0), None);
    assert_eq!(Percent::ratio(5, 0), None);
}

#[test]
fn a_share_rounds_half_up_to_a_hundredth_of_a_percent() {
    let cases: [(u128, u128, u32); 8] = [
        (0, 7, 0),
        (1, 1, 10_000),
        (1, 3, 3_333),
        (2, 3, 6_667),
        // Exactly half a hundredth: 0.005 % and 0.015 % both round up.
        (1, 20_000, 1),
        (3, 20_000, 2),
        // Just under half a hundredth rounds down.
        (1, 20_001, 0),
        (3, 1, 30_000),
    ];
    for (part, whole, hundredths) in cases {
        assert_eq!(
            Percent::ratio(part, whole),
            Some(Percent::from_hundredths(hundredths)),
            "{part}/{whole}"
        );
    }
}

#[test]
fn a_share_too_large_to_hold_saturates() {
    let saturated = Some(Percent::from_hundredths(u32::MAX));
    assert_eq!(
        Percent::ratio(u128::MAX, 1),
        saturated,
        "the whole shares overflow"
    );
    assert_eq!(
        Percent::ratio(u128::from(u32::MAX), 1),
        saturated,
        "the result overflows"
    );
}

/// Parts too large to multiply by `FULL` in `u128` still give their ratio:
/// only a share above `u32::MAX` hundredths saturates, never a large operand.
#[test]
fn a_share_of_operands_too_large_to_scale_is_still_exact() {
    let a_tie = 1_u128 << 122;
    let cases: [(u128, u128, u32); 5] = [
        (u128::MAX / 2, u128::MAX, 5_000),
        (u128::MAX, u128::MAX, 10_000),
        // u128::MAX is divisible by 3.
        (u128::MAX, u128::MAX / 3, 30_000),
        // 2^122 / 2^127 is 3.125 %: exactly half a hundredth, rounded up.
        (a_tie, 1 << 127, 313),
        (a_tie - 1, 1 << 127, 312),
    ];
    for (part, whole, hundredths) in cases {
        assert_eq!(
            Percent::ratio(part, whole),
            Some(Percent::from_hundredths(hundredths)),
            "{part}/{whole}"
        );
    }
}

#[test]
fn of_agrees_with_ratio_for_a_whole_that_is_not_zero() {
    let whole = NonZeroU128::new(3).expect("not zero");
    assert_eq!(Some(Percent::of(1, whole)), Percent::ratio(1, 3));
}

#[test]
fn at_most_caps_a_share_and_keeps_a_smaller_one() {
    let over = Percent::from_hundredths(12_000);
    assert_eq!(over.at_most(Percent::FULL), Percent::FULL);
    let under = Percent::from_hundredths(9_999);
    assert_eq!(under.at_most(Percent::FULL), under);
    assert_eq!(Percent::FULL.hundredths(), 10_000);
}

#[test]
fn an_unmeasured_cpu_share_sorts_before_every_measured_one() {
    assert!(CpuUsage::Pending < CpuUsage::Measured(Percent::from_hundredths(0)));
    assert!(
        CpuUsage::Measured(Percent::from_hundredths(1))
            < CpuUsage::Measured(Percent::from_hundredths(2))
    );
}

#[test]
fn kibibytes_convert_with_a_checked_multiply() {
    assert_eq!(MemoryBytes::from_kib(2).map(MemoryBytes::bytes), Some(2048));
    assert_eq!(MemoryBytes::from_kib(u64::MAX), None);
    assert_eq!(
        MemoryBytes::from_kib(u64::MAX / 1024).map(MemoryBytes::bytes),
        Some(u64::MAX - 1023)
    );
}

#[test]
fn pages_convert_with_a_checked_multiply() {
    assert_eq!(
        MemoryBytes::from_pages(3, 4096).map(MemoryBytes::bytes),
        Some(12_288)
    );
    assert_eq!(MemoryBytes::from_pages(u64::MAX, 2), None);
    assert_eq!(
        MemoryBytes::from_pages(0, 65_536),
        Some(MemoryBytes::from_bytes(0))
    );
}

#[test]
fn a_load_average_keeps_its_three_loads_in_hundredths() {
    let average = LoadAverage::new(
        Load::from_hundredths(52),
        Load::from_hundredths(58),
        Load::from_hundredths(59),
    );
    let loads = [average.one(), average.five(), average.fifteen()];
    assert_eq!(loads.map(Load::hundredths), [52, 58, 59]);
}
