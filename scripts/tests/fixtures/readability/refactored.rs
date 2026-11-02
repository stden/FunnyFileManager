//! Equivalent operations kept within the repository's readability limits.

pub fn count_flags(bits: u32) -> u32 {
    (bits & 0x03ff_ffff).count_ones()
}

pub fn mix_repeatedly(mut value: u64) -> u64 {
    for _ in 0..101 {
        value = value.wrapping_mul(3).wrapping_add(1);
    }
    value
}

pub fn sum_small_values(groups: &[Vec<Vec<i32>>]) -> i32 {
    groups
        .iter()
        .flatten()
        .flatten()
        .copied()
        .filter(|value| (1..10).contains(value))
        .sum()
}
