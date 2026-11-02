//! Purpose: Normalize and range-check multi-edit LSP completions before mutating buffer text.

use crate::lsp::{
    features::TextEdit,
    positions::{self, Range},
};
use std::borrow::Cow;

pub(super) type Edits<'a> = Vec<(std::ops::Range<usize>, Cow<'a, str>)>;

pub(super) fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Whether two edits, in start order, cannot both apply: the later one starts
/// inside the earlier, or where it starts, which leaves their order undefined.
fn clash(earlier: &std::ops::Range<usize>, later: &std::ops::Range<usize>) -> bool {
    let starts_inside = later.start < earlier.end;
    let starts_together = later.start == earlier.start;
    starts_inside || starts_together
}

fn edit_range(text: &str, range: Range) -> Result<std::ops::Range<usize>, &'static str> {
    positions::byte_range(text, range).ok_or("Invalid completion range")
}

pub(super) fn prepare<'a>(
    text: &str,
    primary: &'a TextEdit,
    extra: &'a [TextEdit],
) -> Result<(Edits<'a>, usize), &'static str> {
    let primary_range = edit_range(text, primary.range)?;
    let mut edits = std::iter::once(primary)
        .chain(extra)
        .map(|edit| Ok((edit_range(text, edit.range)?, normalize(&edit.new_text))))
        .collect::<Result<Vec<_>, &'static str>>()?;
    edits.sort_unstable_by_key(|(range, _)| (range.start, range.end));
    if edits
        .iter()
        .zip(edits.iter().skip(1))
        .any(|((earlier, _), (later, _))| clash(earlier, later))
    {
        return Err("Overlapping completion edits");
    }
    let mut caret = primary_range.start + normalize(&primary.new_text).len();
    for (range, replacement) in &edits {
        if range.start < primary_range.start {
            caret = caret - range.len() + replacement.len();
        }
    }
    Ok((edits, caret))
}
