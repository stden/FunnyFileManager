//! Intentionally exceeds each readability limit; never part of a Cargo target.

// test_readability.py compiles this file as a crate root and equivalence.rs
// as `mod complex`; the `path` names the same child file for both.
#[path = "complex/mix.rs"]
mod mix;

pub use mix::mix_repeatedly;

pub fn count_flags(bits: u32) -> u32 {
    let mut count = 0;
    if bits & 0x0000_0001 != 0 {
        count += 1;
    }
    if bits & 0x0000_0002 != 0 {
        count += 1;
    }
    if bits & 0x0000_0004 != 0 {
        count += 1;
    }
    if bits & 0x0000_0008 != 0 {
        count += 1;
    }
    if bits & 0x0000_0010 != 0 {
        count += 1;
    }
    if bits & 0x0000_0020 != 0 {
        count += 1;
    }
    if bits & 0x0000_0040 != 0 {
        count += 1;
    }
    if bits & 0x0000_0080 != 0 {
        count += 1;
    }
    if bits & 0x0000_0100 != 0 {
        count += 1;
    }
    if bits & 0x0000_0200 != 0 {
        count += 1;
    }
    if bits & 0x0000_0400 != 0 {
        count += 1;
    }
    if bits & 0x0000_0800 != 0 {
        count += 1;
    }
    if bits & 0x0000_1000 != 0 {
        count += 1;
    }
    if bits & 0x0000_2000 != 0 {
        count += 1;
    }
    if bits & 0x0000_4000 != 0 {
        count += 1;
    }
    if bits & 0x0000_8000 != 0 {
        count += 1;
    }
    if bits & 0x0001_0000 != 0 {
        count += 1;
    }
    if bits & 0x0002_0000 != 0 {
        count += 1;
    }
    if bits & 0x0004_0000 != 0 {
        count += 1;
    }
    if bits & 0x0008_0000 != 0 {
        count += 1;
    }
    if bits & 0x0010_0000 != 0 {
        count += 1;
    }
    if bits & 0x0020_0000 != 0 {
        count += 1;
    }
    if bits & 0x0040_0000 != 0 {
        count += 1;
    }
    if bits & 0x0080_0000 != 0 {
        count += 1;
    }
    if bits & 0x0100_0000 != 0 {
        count += 1;
    }
    if bits & 0x0200_0000 != 0 {
        count += 1;
    }
    count
}

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
