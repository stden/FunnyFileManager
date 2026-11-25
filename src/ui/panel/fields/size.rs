//! Purpose: Static labels and compact or grouped byte formatting for the Size column.

use std::borrow::Cow;

use super::PARENT_DIR;
use crate::models::{divide_rounded, DirEntry, DirectorySize, EntryKind};

/// Smallest byte count that exceeds the 6-cell Size column (`"999999"` is 6 cells).
const COMPACT_BYTE_THRESHOLD: u64 = 1_000_000;

/// Static label for rows that display a kind word rather than a byte count.
pub(in crate::ui::panel) fn non_numeric_size_label(entry: &DirEntry) -> Option<&'static str> {
    if entry.name == PARENT_DIR {
        return Some("Up");
    }
    match (entry.kind, entry.directory_size) {
        (EntryKind::SymlinkToDirectory, DirectorySize::Unmeasured) => Some("Link"),
        (EntryKind::BrokenSymlink, _) => Some("Broken"),
        (EntryKind::Directory, DirectorySize::Unmeasured) => Some("Folder"),
        (EntryKind::Directory | EntryKind::SymlinkToDirectory, DirectorySize::Measured(_))
        | (EntryKind::File | EntryKind::SymlinkToFile, _) => None,
    }
}

/// Byte count formatted to fit within the 6-cell Size / Allocated column.
pub(in crate::ui::panel) fn fitted_bytes(bytes: u64) -> String {
    if bytes < COMPACT_BYTE_THRESHOLD {
        bytes.to_string()
    } else {
        abbreviate_bytes(bytes)
    }
}

/// The Size column's text: Far names what a row *is* before it measures it.
pub(in crate::ui) fn size_field(entry: &DirEntry) -> Cow<'static, str> {
    if let Some(label) = non_numeric_size_label(entry) {
        return Cow::Borrowed(label);
    }
    Cow::Owned(fitted_bytes(entry.size.bytes()))
}

/// Far's compact byte count — "381 K", "49.4 G".
pub(in crate::ui) fn abbreviate_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["K", "M", "G", "T"];

    let bytes = u128::from(bytes);
    let mut scale = 1;
    let mut unit = None;
    for next in UNITS {
        if divide_rounded(bytes, scale) < 1000 {
            break;
        }
        scale *= 1024;
        unit = Some(next);
    }
    let Some(unit) = unit else {
        return bytes.to_string();
    };
    let tenths = divide_rounded(bytes * 10, scale);
    if tenths >= 1000 {
        format!("{} {unit}", divide_rounded(bytes, scale))
    } else {
        format!("{}.{} {unit}", tenths / 10, tenths % 10)
    }
}

/// `value` with Far's thousands separators: `5,368,709,120`.
pub(in crate::ui) fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
