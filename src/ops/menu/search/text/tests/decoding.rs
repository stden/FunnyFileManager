//! Purpose: Preserve lossy decoding and Unicode matching under arbitrary read boundaries.

use super::super::{decode_complete, fold_into, push_chunk};
use super::*;

struct ShortReads<'a> {
    bytes: &'a [u8],
    limit: usize,
}

impl Read for ShortReads<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let limit = self.limit.min(buffer.len());
        self.bytes.read(&mut buffer[..limit])
    }
}

#[test]
fn short_reads_match_the_complete_lossy_text() {
    let mut bytes = "PREFIX Ж İΣ 😀".as_bytes().to_vec();
    bytes.extend_from_slice(b"bad\xfftail \xe2( \xe2\x82");
    let needle = fold(&String::from_utf8_lossy(&bytes));
    for limit in [1, 2, 3, 5, 7] {
        let reader = ShortReads {
            bytes: &bytes,
            limit,
        };
        assert_eq!(
            contains_text(reader, &needle, later()).unwrap(),
            Scan::Found
        );
        let reader = ShortReads {
            bytes: &bytes,
            limit,
        };
        assert_eq!(
            contains_text(reader, "absent", later()).unwrap(),
            Scan::Absent
        );
    }
}

#[test]
fn invalid_bytes_before_an_incomplete_suffix_keep_both_replacements() {
    assert_eq!(scan(b"A\xffB\xe2\x82", "a\u{FFFD}b\u{FFFD}"), Scan::Found);
}

#[test]
fn ascii_and_expanding_unicode_match_across_chunks() {
    let bytes = padded(CHUNK - 1, "İΣASCII");
    assert_eq!(scan(&bytes, "Ai\u{307}σascii"), Scan::Found);
}

#[test]
fn sigma_equivalence_survives_a_chunk_boundary() {
    let bytes = padded(CHUNK - 2, "ΟΣ");
    assert_eq!(scan(&bytes, "ος"), Scan::Found);
}

#[test]
fn complete_utf8_chunks_reuse_output_without_a_decoding_buffer() {
    for (text, expected) in [
        ("ASCII content", "ascii content"),
        ("ΣİЖ😀", "σi\u{307}ж😀"),
    ] {
        let bytes = text.as_bytes().to_vec();
        let mut pending = Vec::new();
        let mut out = String::with_capacity(CHUNK);
        let capacity = out.capacity();
        push_chunk(bytes.len(), &bytes, &mut pending, &mut out);
        assert_eq!(out, expected);
        assert_eq!(out.capacity(), capacity);
        assert_eq!(pending.capacity(), 0);
    }
}

#[test]
fn invalid_prefix_is_consumed_and_an_incomplete_character_is_retained() {
    let mut bytes = b"A\xffB\xd0".to_vec();
    let mut out = String::new();
    decode_complete(&mut bytes, &mut out);
    assert_eq!(out, "a\u{FFFD}b");
    assert_eq!(bytes, b"\xd0");
    bytes.push(0x96);
    decode_complete(&mut bytes, &mut out);
    assert_eq!(out, "a\u{FFFD}bж");
    assert_eq!(bytes.len(), 0);
}

#[test]
fn an_invalid_sequence_before_ascii_is_replaced_immediately() {
    let mut bytes = b"\xe2(".to_vec();
    let mut out = String::new();
    decode_complete(&mut bytes, &mut out);
    assert_eq!(out, "\u{FFFD}(");
    assert_eq!(bytes.len(), 0);
}

#[test]
fn folding_appends_without_lowercasing_the_existing_prefix() {
    let mut out = String::from("Prefix Ж ");
    fold_into("ABC", &mut out);
    fold_into("İΣ", &mut out);
    fold_into("XYZ", &mut out);
    assert_eq!(out, "Prefix Ж abci\u{307}σxyz");
}

#[test]
fn non_ascii_code_points_only_fold_to_ascii_i_or_k() {
    for ch in '\u{80}'..=char::MAX {
        for lower in ch.to_lowercase() {
            assert!(
                !lower.is_ascii() || lower == 'i' || lower == 'k',
                "unexpected ASCII fold {ch:?} -> {lower:?}"
            );
        }
    }
}

#[test]
fn kelvin_and_split_anchor_across_chunk_boundaries_are_found() {
    assert_eq!(scan("temp=273\u{212A}!".as_bytes(), "273k!"), Scan::Found);
    assert_eq!(scan(&padded(CHUNK - 4, "abc_def"), "abc_def"), Scan::Found);
    assert_eq!(scan(&padded(CHUNK - 3, "abc_def"), "abc_def"), Scan::Found);
}
