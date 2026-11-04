//! Purpose: Safe input encoding and region post-processing around Colorer's C API.
//!
//! Colorer reports overlapping UTF-16 regions — a bracket is both `def:Symbol`
//! and `def:PairStart` — so a line's entry is a list where the *last* region to
//! cover a character wins, converted from UTF-16 code units to `char` indices.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::Path;

use super::{region_token, Span};

pub(super) type EncodedInputs = (CString, Vec<CString>, Vec<*const c_char>);

/// Filter out Colorer's `"default"` sentinel, which means "no scheme matched".
pub(super) fn non_default_type(raw: &CStr) -> Option<String> {
    let chosen = raw.to_string_lossy().into_owned();
    (chosen != "default").then_some(chosen)
}

/// Encode `path` and `lines` as NUL-terminated strings for `colorer_parse`.
///
/// The shim reads `const char*`, so every line needs a NUL. A line with an
/// interior NUL cannot be passed, and is sent as empty rather than truncating
/// the file at that point.
pub(super) fn encode_inputs(path: &Path, lines: &[String]) -> Option<EncodedInputs> {
    let name = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    let owned: Vec<CString> = lines
        .iter()
        .map(|line| CString::new(line.as_str()).unwrap_or_default())
        .collect();
    let pointers: Vec<*const c_char> = owned.iter().map(|line| line.as_ptr()).collect();
    Some((name, owned, pointers))
}

/// Convert every collected region's UTF-16 offsets into Rust `char` indices.
pub(super) fn utf16_spans_to_chars(lines: &[String], collected: &mut [Vec<Span>]) {
    for (line, row) in lines.iter().zip(collected) {
        if line.is_ascii() {
            for span in row {
                span.start = span.start.min(line.len());
                span.end = span.end.min(line.len());
            }
            continue;
        }
        for span in row {
            span.start = crate::highlight::regions::utf16_to_char_idx(line, span.start);
            span.end = crate::highlight::regions::utf16_to_char_idx(line, span.end);
        }
    }
}

/// Validate and append one callback region onto `collected`.
///
/// Anything out of bounds, non-UTF-8, negative, or empty is dropped rather than
/// guessed at — a region on a line that does not exist is a bug in the shim,
/// not something to paint.
pub(super) fn push_region(
    collected: &mut [Vec<Span>],
    line: usize,
    sx: c_int,
    ex: c_int,
    raw_name: &CStr,
) {
    let Some(row) = collected.get_mut(line) else {
        return;
    };
    let Ok(name) = raw_name.to_str() else {
        return;
    };
    let (Ok(start), Ok(end)) = (usize::try_from(sx), usize::try_from(ex)) else {
        return;
    };
    if end <= start {
        return;
    }
    row.push(Span {
        start,
        end,
        token: region_token(name),
    });
}

#[cfg(test)]
mod tests;
