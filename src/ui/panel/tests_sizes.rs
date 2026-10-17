//! Purpose: Pin the footer's compact byte counts to the float formatting they replaced.

use super::abbreviate_bytes;

/// The float version: divide by 1024 while at least 1000, then `{:.0}` or
/// `{:.1}`. `f64::from(u32)` keeps it exact, so it is a faithful reference.
fn float_abbreviation(bytes: u32) -> String {
    const UNITS: [&str; 5] = ["", "K", "M", "G", "T"];
    let mut value = f64::from(bytes);
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        bytes.to_string()
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A stride that is not a power of two walks the rounding residues; strides of
/// 256 and 2^18 land on the exact `.x5` ties in the K and M ranges.
#[test]
fn abbreviated_sizes_match_the_float_formatting_below_four_gibibytes() {
    let samples = (0..u32::MAX)
        .step_by(65_537)
        .chain((0..1 << 24).step_by(256))
        .chain((0..u32::MAX).step_by(1 << 18));
    for bytes in samples {
        assert_eq!(
            abbreviate_bytes(u64::from(bytes)),
            float_abbreviation(bytes),
            "{bytes}"
        );
    }
}

#[test]
fn abbreviated_sizes_stay_in_terabytes_at_the_top_of_u64() {
    assert_eq!(abbreviate_bytes(u64::MAX), "16777216 T");
    assert_eq!(abbreviate_bytes(999), "999");
    assert_eq!(abbreviate_bytes(1000), "1.0 K");
}

#[test]
fn measured_directory_shows_its_byte_size_instead_of_folder_label() {
    use super::size_field;
    use crate::models::{DirEntry, EntryKind, FileSize};

    let mut dir = DirEntry {
        name: "docs".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    };
    assert_eq!(size_field(&dir), "Folder");

    dir.set_measured_size(FileSize::from_bytes(12_345));
    assert_eq!(size_field(&dir), "12345");

    dir.set_measured_size(FileSize::from_bytes(5_242_880));
    assert_eq!(size_field(&dir), "5.0 M");
}
