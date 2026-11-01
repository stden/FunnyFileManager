//! Unit tests: attribute rendering on both platforms, timestamp fallbacks.

use super::*;

#[test]
fn mode_bits_render_like_ls() {
    assert_eq!(Attributes::from_bits(0o100_644).mode_string(), "-rw-r--r--");
    assert_eq!(Attributes::from_bits(0o040_755).mode_string(), "drwxr-xr-x");
    assert_eq!(Attributes::from_bits(0o120_777).mode_string(), "lrwxrwxrwx");
    assert_eq!(Attributes::from_bits(0o104_755).mode_string(), "-rwsr-xr-x");
    assert_eq!(Attributes::from_bits(0o102_644).mode_string(), "-rw-r-Sr--");
    assert_eq!(Attributes::from_bits(0o041_777).mode_string(), "drwxrwxrwt");
    assert_eq!(Attributes::from_bits(0o000_000).mode_string(), "?---------");
}

#[test]
fn windows_flags_render_as_far_letters_in_order() {
    assert_eq!(Attributes::from_bits(0x27).flag_letters(), "RHSA");
    assert_eq!(Attributes::from_bits(0x21).flag_letters(), "RA");
    assert_eq!(Attributes::from_bits(0x10).flag_letters(), "");
}

#[test]
fn display_uses_the_platform_renderer() {
    let attributes = Attributes::from_bits(if cfg!(windows) { 0x21 } else { 0o100_644 });
    let expected = if cfg!(windows) { "RA" } else { "-rw-r--r--" };
    assert_eq!(attributes.to_string(), expected);
    assert_eq!(attributes.bits(), attributes.bits());
}

#[test]
fn a_refused_timestamp_is_unknown_and_sorts_first() {
    let refused = Modified::from_result(Err(std::io::Error::other("no ctime")));
    assert_eq!(refused, Modified::UNKNOWN);
    assert_eq!(refused.date().len(), 0);
    let known = Modified::new(std::time::UNIX_EPOCH);
    assert!(refused < known);
}

#[test]
fn a_default_entry_carries_the_neutral_facts() {
    let entry = DirEntry::default();
    assert_eq!(entry.links, 0);
    assert!(entry.owner.is_none());
    assert!(entry.description.is_none());
    assert_eq!(entry.attributes, Attributes::default());
}

// ── FileSize: integer scaling ────────────────────────────────────────────

#[test]
fn divide_rounded_rounds_to_nearest_and_breaks_ties_to_even() {
    assert_eq!(divide_rounded(0, 1), 0);
    assert_eq!(divide_rounded(14, 10), 1);
    assert_eq!(divide_rounded(16, 10), 2);
    assert_eq!(divide_rounded(5, 10), 0, "a tie keeps an even quotient");
    assert_eq!(divide_rounded(15, 10), 2, "a tie rounds an odd quotient up");
    assert_eq!(divide_rounded(25, 10), 2);
}

#[test]
fn divide_rounded_does_not_overflow_at_the_ends_of_u128() {
    assert_eq!(divide_rounded(u128::MAX, 1), u128::MAX);
    assert_eq!(divide_rounded(u128::MAX, u128::MAX), 1);
    assert_eq!(divide_rounded(u128::MAX - 1, u128::MAX), 1);
    assert_eq!(divide_rounded(u128::MAX, 2), 1 << 127);
}

/// The rate the progress dialog shows: one decimal, in `Display`'s units.
#[test]
fn one_decimal_keeps_one_decimal_in_the_units_of_display() {
    assert_eq!(FileSize::from_bytes(0).one_decimal(), "0 B");
    assert_eq!(FileSize::from_bytes(1023).one_decimal(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).one_decimal(), "1.0 KB");
    assert_eq!(FileSize::from_bytes(95_334).one_decimal(), "93.1 KB");
    assert_eq!(FileSize::from_bytes(47_125_964).one_decimal(), "44.9 MB");
}

/// Integer scaling prints exactly what `{:.2}` of the float quotient printed,
/// including the exact ties (x.125, x.375, ...) that it breaks to even.
/// `f64::from(u32)` keeps the reference exact.
#[test]
fn file_size_prints_what_the_float_formatting_printed() {
    let ranges = [
        (1_u32 << 10, 1 << 20, "KB"),
        (1 << 20, 1 << 30, "MB"),
        (1 << 30, u32::MAX, "GB"),
    ];
    for (unit, end, name) in ranges {
        let step = usize::try_from(unit / 64).expect("step fits usize");
        for size in (unit..end).step_by(step).flat_map(|size| [size, size + 1]) {
            let float = format!("{:.2} {name}", f64::from(size) / f64::from(unit));
            assert_eq!(
                FileSize::from_bytes(u64::from(size)).to_string(),
                float,
                "{size}"
            );
        }
    }
}

#[test]
fn megabytes_is_exact_and_saturates_at_u32_max_mebibytes() {
    const MIB: u64 = 1 << 20;
    const ONE_BYTE: f64 = 1.0 / 1_048_576.0;
    assert_eq!(
        FileSize::from_bytes(0).megabytes().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(
        FileSize::from_bytes(1).megabytes().to_bits(),
        ONE_BYTE.to_bits()
    );
    assert_eq!(
        FileSize::from_bytes(MIB + MIB / 2).megabytes().to_bits(),
        1.5_f64.to_bits()
    );
    let largest = u64::from(u32::MAX) * MIB + (MIB - 1);
    let exact = f64::from(u32::MAX) + (1.0 - ONE_BYTE);
    assert_eq!(
        FileSize::from_bytes(largest).megabytes().to_bits(),
        exact.to_bits()
    );
    let saturated = f64::from(u32::MAX).to_bits();
    assert_eq!(
        FileSize::from_bytes(largest + 1).megabytes().to_bits(),
        saturated
    );
    assert_eq!(
        FileSize::from_bytes(u64::MAX).megabytes().to_bits(),
        saturated
    );
}
