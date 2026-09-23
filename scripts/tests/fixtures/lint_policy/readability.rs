//! Negative: functions past each readability limit in the shared `clippy.toml`.
//!
//! Clippy's default nesting limit is off, so `excessive_nesting` also proves
//! that the crate found `clippy.toml` from its location under `target/`.

/// Thirty branches: over the cognitive-complexity and line limits.
#[must_use]
pub const fn count_flags(bits: u32) -> u32 {
    let mut count = 0;
    if bits & (1 << 0) != 0 { count += 1; }
    if bits & (1 << 1) != 0 { count += 1; }
    if bits & (1 << 2) != 0 { count += 1; }
    if bits & (1 << 3) != 0 { count += 1; }
    if bits & (1 << 4) != 0 { count += 1; }
    if bits & (1 << 5) != 0 { count += 1; }
    if bits & (1 << 6) != 0 { count += 1; }
    if bits & (1 << 7) != 0 { count += 1; }
    if bits & (1 << 8) != 0 { count += 1; }
    if bits & (1 << 9) != 0 { count += 1; }
    if bits & (1 << 10) != 0 { count += 1; }
    if bits & (1 << 11) != 0 { count += 1; }
    if bits & (1 << 12) != 0 { count += 1; }
    if bits & (1 << 13) != 0 { count += 1; }
    if bits & (1 << 14) != 0 { count += 1; }
    if bits & (1 << 15) != 0 { count += 1; }
    if bits & (1 << 16) != 0 { count += 1; }
    if bits & (1 << 17) != 0 { count += 1; }
    if bits & (1 << 18) != 0 { count += 1; }
    if bits & (1 << 19) != 0 { count += 1; }
    if bits & (1 << 20) != 0 { count += 1; }
    if bits & (1 << 21) != 0 { count += 1; }
    if bits & (1 << 22) != 0 { count += 1; }
    if bits & (1 << 23) != 0 { count += 1; }
    if bits & (1 << 24) != 0 { count += 1; }
    if bits & (1 << 25) != 0 { count += 1; }
    if bits & (1 << 26) != 0 { count += 1; }
    if bits & (1 << 27) != 0 { count += 1; }
    if bits & (1 << 28) != 0 { count += 1; }
    if bits & (1 << 29) != 0 { count += 1; }
    count
}

/// Five nested blocks: over the nesting limit.
#[must_use]
pub fn sum_small_values(groups: &[Vec<Vec<i32>>]) -> i32 {
    let mut total = 0;
    for group in groups {
        for row in group {
            for value in row {
                if (1..10).contains(value) {
                    total += *value;
                }
            }
        }
    }
    total
}
