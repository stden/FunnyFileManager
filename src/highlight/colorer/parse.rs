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
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::path::Path;

    fn cstr(s: &str) -> std::ffi::CString {
        CString::new(s).expect("test CStr")
    }

    #[test]
    fn non_default_type_passes_through_non_default_names() {
        let name = cstr("rust");
        assert_eq!(non_default_type(&name), Some("rust".to_owned()));
    }

    #[test]
    fn non_default_type_rejects_the_default_sentinel() {
        let name = cstr("default");
        assert_eq!(non_default_type(&name), None);
    }

    #[test]
    fn encode_inputs_returns_one_pointer_per_line() {
        let lines: Vec<String> = vec!["fn main() {}".to_owned(), "let x = 1;".to_owned()];
        let (name, owned, ptrs) = encode_inputs(Path::new("main.rs"), &lines).expect("encode");
        assert!(!name.as_bytes().is_empty());
        assert_eq!(owned.len(), 2);
        assert_eq!(ptrs.len(), 2);
    }

    #[test]
    fn encode_inputs_replaces_null_byte_lines_with_empty() {
        // A line containing a NUL byte must not abort the encoding.
        let lines = vec!["ok".to_owned(), "bad\0byte".to_owned(), "ok".to_owned()];
        let (_name, owned, _ptrs) = encode_inputs(Path::new("f.rs"), &lines).expect("encode");
        // The NUL-byte line becomes an empty CString (Colorer sees "").
        assert_eq!(owned[1].as_bytes(), b"");
    }

    #[test]
    fn push_region_appends_a_valid_span() {
        let mut rows: Vec<Vec<Span>> = vec![Vec::new(); 2];
        push_region(&mut rows, 0, 2, 5, &cstr("def:Keyword"));
        assert_eq!(rows[0].len(), 1);
        assert_eq!((rows[0][0].start, rows[0][0].end), (2, 5));
    }

    #[test]
    fn push_region_drops_out_of_bounds_line() {
        let mut rows: Vec<Vec<Span>> = vec![Vec::new(); 1];
        push_region(&mut rows, 99, 0, 1, &cstr("def:Keyword"));
        assert!(rows[0].is_empty());
    }

    #[test]
    fn push_region_drops_empty_or_reversed_spans() {
        let mut rows: Vec<Vec<Span>> = vec![Vec::new(); 1];
        push_region(&mut rows, 0, 3, 3, &cstr("def:Keyword")); // end == start
        push_region(&mut rows, 0, 5, 2, &cstr("def:Keyword")); // end < start
        assert!(rows[0].is_empty());
    }

    #[test]
    fn utf16_spans_to_chars_converts_ascii_offsets_unchanged() {
        let lines = vec!["hello".to_owned()];
        let token = crate::highlight::Token::Keyword;
        let mut rows: Vec<Vec<Span>> = vec![vec![Span { start: 1, end: 3, token }]];
        utf16_spans_to_chars(&lines, &mut rows);
        // Pure ASCII: UTF-16 units == char indices.
        assert_eq!((rows[0][0].start, rows[0][0].end), (1, 3));
    }
}
