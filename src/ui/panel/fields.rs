//! Purpose: The text of a panel's cells, measured in terminal cells: a name
//! split at its extension and clipped with Far's `}`, padding and alignment,
//! and the Size column's words and compact byte counts.

mod size;

use super::super::cells::clip::take_cells_str;
use super::super::Cells;
use crate::models::DirEntry;

pub(in crate::ui) use size::{abbreviate_bytes, group_digits, size_field};
pub(in crate::ui::panel) use size::{fitted_bytes, non_numeric_size_label};

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

struct ClippedText<'a> {
    prefix: &'a str,
    truncation_mark: &'static str,
    used: Cells,
}

impl<'a> ClippedText<'a> {
    fn fit(text: &'a str, width: Cells, truncation_mark: &'static str) -> Self {
        let cells = Cells::of(text);
        if cells <= width {
            return Self {
                prefix: text,
                truncation_mark: "",
                used: cells,
            };
        }
        if width.is_zero() {
            return Self {
                prefix: "",
                truncation_mark: "",
                used: Cells::ZERO,
            };
        }
        let (prefix, used) = take_cells_str(text, width.saturating_sub(Cells::new(1)));
        Self {
            prefix,
            truncation_mark,
            used: used + Cells::new(1),
        }
    }

    const fn byte_len(&self) -> usize {
        self.prefix.len() + self.truncation_mark.len()
    }

    fn push_to(&self, out: &mut String) {
        out.push_str(self.prefix);
        out.push_str(self.truncation_mark);
    }
}

fn pad_clipped(text: &str, width: Cells, truncation_mark: &'static str) -> String {
    let clipped = ClippedText::fit(text, width, truncation_mark);
    let pad = width.saturating_sub(clipped.used).get();
    let mut out = String::with_capacity(clipped.byte_len() + pad);
    clipped.push_to(&mut out);
    push_spaces(&mut out, pad);
    out
}

/// First `width` cells of a panel name, marked with Far's `}` when cut.
pub(in crate::ui) fn clip_name(name: &str, width: Cells) -> String {
    let clipped = ClippedText::fit(name, width, "}");
    let mut out = String::with_capacity(clipped.byte_len());
    clipped.push_to(&mut out);
    out
}

/// `name` clipped with Far's `}` and padded with spaces to `width` in one allocation.
pub(in crate::ui) fn pad_clipped_name(name: &str, width: Cells) -> String {
    pad_clipped(name, width, "}")
}

/// `text` clipped with `…` and padded with spaces to `width` in one allocation.
pub(in crate::ui) fn pad_head_cells(text: &str, width: Cells) -> String {
    pad_clipped(text, width, "…")
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
    let clipped = ClippedText::fit(text, width, "…");
    let pad = width.saturating_sub(clipped.used).get();
    let mut out = String::with_capacity(pad + clipped.byte_len());
    push_spaces(&mut out, pad);
    clipped.push_to(&mut out);
    out
}

/// `text` centred in exactly `width` cells — the column headers' alignment.
pub(in crate::ui) fn center_cells(text: &str, width: Cells) -> String {
    let clipped = ClippedText::fit(text, width, "…");
    let slack = width.saturating_sub(clipped.used).get();
    let left = slack / 2;
    let mut out = String::with_capacity(slack + clipped.byte_len());
    push_spaces(&mut out, left);
    clipped.push_to(&mut out);
    push_spaces(&mut out, slack - left);
    out
}

/// The name column for one row: base name left, extension right.
pub(in crate::ui) fn name_field(entry: &DirEntry, width: Cells) -> String {
    let (base, extension) = split_extension(entry);
    let extension_width = Cells::of(extension);
    if extension.is_empty() || extension_width + Cells::new(2) > width {
        return pad_clipped_name(&entry.name, width);
    }
    let base_room = width.saturating_sub(extension_width + Cells::new(1));
    let clipped = ClippedText::fit(base, base_room, "}");
    let gap = width.saturating_sub(clipped.used + extension_width).get();
    let mut out = String::with_capacity(clipped.byte_len() + gap + extension.len());
    clipped.push_to(&mut out);
    push_spaces(&mut out, gap);
    out.push_str(extension);
    out
}
