//! Purpose: The text of a panel's cells, measured in terminal cells: a name
//! split at its extension and clipped with Far's `}`, padding and alignment,
//! and the Size column's words and compact byte counts.

use super::super::{head_cells, take_cells, Cells};
use super::columns::SIZE_COLUMN_WIDTH;
use crate::models::{divide_rounded, DirEntry, EntryKind};

/// The row that leads out of a directory, which the Size column labels "Up".
const PARENT_DIR: &str = "..";

/// Far splits a file's name at its final dot and right-aligns the extension
/// against the Size column, so a listing lines up by type without sorting.
///
/// Directories are never split — `.cargo` is a name, not an extension — and
/// neither is a leading dot, which starts a hidden file rather than ending an
/// empty one.
pub(in crate::ui) fn split_extension(entry: &DirEntry) -> (&str, &str) {
    if entry.kind.is_dir() {
        return (&entry.name, "");
    }
    match entry.name.rsplit_once('.') {
        Some(("", _)) | None => (&entry.name, ""),
        Some((stem, extension)) => (stem, extension),
    }
}

/// First `width` cells of a panel name, marked with Far's `}` when cut.
///
/// A distinct marker from [`head_cells`]' ellipsis on purpose: `}` cannot
/// occur mid-name often enough to be mistaken for one, and it is the glyph a
/// Far user already reads as "there is more here".
pub(in crate::ui) fn clip_name(name: &str, width: Cells) -> String {
    if Cells::of(name) <= width {
        return name.to_owned();
    }
    if width.is_zero() {
        return String::new();
    }
    let (mut cut, _) = take_cells(name, width.saturating_sub(Cells(1)));
    cut.push('}');
    cut
}

/// `text` padded with blanks to exactly `width` cells.
pub(in crate::ui) fn pad_cells(text: &str, width: Cells) -> String {
    let padding = " ".repeat(width.saturating_sub(Cells::of(text)).get());
    format!("{text}{padding}")
}

/// `text` right-aligned in exactly `width` cells, as every column but the name
/// is. `pub(super)` because the drive dialog aligns its size columns the same
/// way rather than keeping a copy.
pub(in crate::ui) fn right_cells(text: &str, width: Cells) -> String {
    let fitted = head_cells(text, width);
    let padding = " ".repeat(width.saturating_sub(Cells::of(&fitted)).get());
    format!("{padding}{fitted}")
}

/// `text` centred in exactly `width` cells — the column headers' alignment.
pub(in crate::ui) fn center_cells(text: &str, width: Cells) -> String {
    let fitted = head_cells(text, width);
    let slack = width.saturating_sub(Cells::of(&fitted)).get();
    let left = slack / 2;
    format!("{}{fitted}{}", " ".repeat(left), " ".repeat(slack - left))
}

/// The name column for one row: base name left, extension right.
pub(in crate::ui) fn name_field(entry: &DirEntry, width: Cells) -> String {
    let (base, extension) = split_extension(entry);
    let extension_width = Cells::of(extension);
    // Splitting costs the extension plus at least one blank column. Where the
    // panel cannot pay that, the undivided name keeps the room instead: a bare
    // right-aligned "rs" identifies a file far less than a truncated name does.
    if extension.is_empty() || extension_width + Cells(2) > width {
        return pad_cells(&clip_name(&entry.name, width), width);
    }
    let base_room = width.saturating_sub(extension_width + Cells(1));
    let base = clip_name(base, base_room);
    let gap = width
        .saturating_sub(Cells::of(&base) + extension_width)
        .get();
    format!("{base}{}{extension}", " ".repeat(gap))
}

/// The Size column's text: Far names what a row *is* before it measures it.
pub(in crate::ui) fn size_field(entry: &DirEntry) -> String {
    if entry.name == PARENT_DIR {
        return "Up".to_owned();
    }
    // A link to a folder is entered like one, but it is not one: it says so
    // where a folder says "Folder". A link to a file shows its target's size.
    match (entry.kind, entry.directory_size) {
        (EntryKind::SymlinkToDirectory, crate::models::DirectorySize::Unmeasured) => {
            return "Link".to_owned()
        }
        (EntryKind::BrokenSymlink, _) => return "Broken".to_owned(),
        (EntryKind::Directory, crate::models::DirectorySize::Unmeasured) => {
            return "Folder".to_owned()
        }
        (
            EntryKind::Directory | EntryKind::SymlinkToDirectory,
            crate::models::DirectorySize::Measured(_),
        )
        | (EntryKind::File | EntryKind::SymlinkToFile, _) => {}
    }
    let bytes = entry.size.bytes();
    let exact = bytes.to_string();
    // Far prints the byte count itself and only abbreviates what will not fit,
    // so the common case stays a number the user can compare digit by digit.
    if Cells::of(&exact) <= SIZE_COLUMN_WIDTH {
        exact
    } else {
        abbreviate_bytes(bytes)
    }
}

/// Far's compact byte count — "381 K", "49.4 G": the largest unit that keeps
/// the value under 1000, carrying a decimal only while there is room for one.
pub(in crate::ui) fn abbreviate_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["K", "M", "G", "T"];

    // Integer maths, exact for every u64: the value shown is `bytes / scale`.
    let bytes = u128::from(bytes);
    let mut scale = 1;
    let mut unit = None;
    for next in UNITS {
        if bytes < 1000 * scale {
            break;
        }
        scale *= 1024;
        unit = Some(next);
    }
    let Some(unit) = unit else {
        return bytes.to_string();
    };
    if bytes >= 100 * scale {
        format!("{} {unit}", divide_rounded(bytes, scale))
    } else {
        let tenths = divide_rounded(bytes * 10, scale);
        format!("{}.{} {unit}", tenths / 10, tenths % 10)
    }
}

/// `value` with Far's thousands separators: `5,368,709,120`.
pub(in crate::ui) fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
