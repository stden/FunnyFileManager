//! Purpose: The text of a panel's cells, measured in terminal cells: a name
//! split at its extension and clipped with Far's `}`, padding and alignment,
//! and the Size column's words and compact byte counts.

use super::super::{head_cells, take_cells, Cells};
use super::columns::SIZE_COLUMN_WIDTH;
use crate::models::{divide_rounded, DirEntry, EntryKind};

/// The row that leads out of a directory, which the Size column labels "Up".
const PARENT_DIR: &str = "..";
const SPACES: &str = "                                                                ";

pub(super) fn push_spaces(out: &mut String, mut count: usize) {
    while count > 0 {
        let chunk = count.min(SPACES.len());
        out.push_str(SPACES.get(..chunk).unwrap_or(""));
        count -= chunk;
    }
}

/// Far splits a file's name at its final dot and right-aligns the extension
/// against the Size column, so a listing lines up by type without sorting.
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
    let pad = width.saturating_sub(Cells::of(text)).get();
    let mut out = String::with_capacity(text.len() + pad);
    out.push_str(text);
    push_spaces(&mut out, pad);
    out
}

/// `text` right-aligned in exactly `width` cells.
pub(in crate::ui) fn right_cells(text: &str, width: Cells) -> String {
    let cells = Cells::of(text);
    if cells <= width {
        let pad = width.saturating_sub(cells).get();
        let mut out = String::with_capacity(pad + text.len());
        push_spaces(&mut out, pad);
        out.push_str(text);
        return out;
    }
    let fitted = head_cells(text, width);
    let pad = width.saturating_sub(Cells::of(&fitted)).get();
    let mut out = String::with_capacity(pad + fitted.len());
    push_spaces(&mut out, pad);
    out.push_str(&fitted);
    out
}

/// `text` centred in exactly `width` cells — the column headers' alignment.
pub(in crate::ui) fn center_cells(text: &str, width: Cells) -> String {
    let cells = Cells::of(text);
    if cells <= width {
        let slack = width.saturating_sub(cells).get();
        let left = slack / 2;
        let mut out = String::with_capacity(slack + text.len());
        push_spaces(&mut out, left);
        out.push_str(text);
        push_spaces(&mut out, slack - left);
        return out;
    }
    let fitted = head_cells(text, width);
    let slack = width.saturating_sub(Cells::of(&fitted)).get();
    let left = slack / 2;
    let mut out = String::with_capacity(slack + fitted.len());
    push_spaces(&mut out, left);
    out.push_str(&fitted);
    push_spaces(&mut out, slack - left);
    out
}

/// The name column for one row: base name left, extension right.
pub(in crate::ui) fn name_field(entry: &DirEntry, width: Cells) -> String {
    let (base, extension) = split_extension(entry);
    let extension_width = Cells::of(extension);
    if extension.is_empty() || extension_width + Cells(2) > width {
        let name_cells = Cells::of(&entry.name);
        if name_cells <= width {
            return pad_cells(&entry.name, width);
        }
        return pad_cells(&clip_name(&entry.name, width), width);
    }
    let base_room = width.saturating_sub(extension_width + Cells(1));
    let base_cells = Cells::of(base);
    if base_cells <= base_room {
        let gap = width.saturating_sub(base_cells + extension_width).get();
        let mut out = String::with_capacity(base.len() + gap + extension.len());
        out.push_str(base);
        push_spaces(&mut out, gap);
        out.push_str(extension);
        return out;
    }
    let clipped = clip_name(base, base_room);
    let gap = width
        .saturating_sub(Cells::of(&clipped) + extension_width)
        .get();
    let mut out = String::with_capacity(clipped.len() + gap + extension.len());
    out.push_str(&clipped);
    push_spaces(&mut out, gap);
    out.push_str(extension);
    out
}

/// The Size column's text: Far names what a row *is* before it measures it.
pub(in crate::ui) fn size_field(entry: &DirEntry) -> String {
    if entry.name == PARENT_DIR {
        return "Up".to_owned();
    }
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
    let _ = SIZE_COLUMN_WIDTH;
    if bytes < 1_000_000 {
        bytes.to_string()
    } else {
        abbreviate_bytes(bytes)
    }
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
