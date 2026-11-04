//! Purpose: Unit tests for Colorer input encoding and UTF-16 region post-processing.

use super::*;

fn cstr(s: &str) -> CString {
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
    assert_ne!(name.as_bytes(), b"");
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
    assert_eq!(rows[0].len(), 0);
}

#[test]
fn push_region_drops_empty_or_reversed_spans() {
    let mut rows: Vec<Vec<Span>> = vec![Vec::new(); 1];
    push_region(&mut rows, 0, 3, 3, &cstr("def:Keyword")); // end == start
    push_region(&mut rows, 0, 5, 2, &cstr("def:Keyword")); // end < start
    assert_eq!(rows[0].len(), 0);
}

#[test]
fn utf16_spans_to_chars_converts_ascii_and_multibyte_offsets() {
    let lines = vec!["hello".to_owned(), "a🦀b".to_owned()];
    let token = crate::highlight::Token::Keyword;
    let mut rows: Vec<Vec<Span>> = vec![
        vec![Span {
            start: 1,
            end: 3,
            token,
        }],
        vec![Span {
            start: 1,
            end: 3,
            token,
        }],
    ];
    utf16_spans_to_chars(&lines, &mut rows);
    assert_eq!((rows[0][0].start, rows[0][0].end), (1, 3));
    assert_eq!((rows[1][0].start, rows[1][0].end), (1, 2));
}
