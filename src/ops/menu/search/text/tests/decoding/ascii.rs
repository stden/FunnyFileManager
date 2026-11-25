//! The ASCII-query shortcut must preserve Unicode and lossy-read semantics.

use super::*;

#[test]
fn ascii_queries_match_the_lossy_reference_at_every_short_read_boundary() {
    let mut bytes = "Σ Ж 日本語 İX \u{212A}EY NEEDLE İ\u{212A}"
        .as_bytes()
        .to_vec();
    bytes.extend_from_slice(b"\xffEND\xc4\xe2\x84");
    let reference = fold(&String::from_utf8_lossy(&bytes));
    for needle in ["i", "ix", "key", "needle", "end", "ik", "Σ", "İ", "missing"] {
        let needle = fold(needle);
        let expected = if reference.contains(&needle) {
            Scan::Found
        } else {
            Scan::Absent
        };
        for limit in [1, 2, 3, 5, 7, 64] {
            let reader = ShortReads {
                bytes: &bytes,
                limit,
            };
            assert_eq!(contains_text(reader, &needle, later()).unwrap(), expected);
        }
    }
}

#[test]
fn unicode_ascii_mappings_survive_real_chunk_boundaries() {
    for padding in [CHUNK - 1, CHUNK - 2, CHUNK - 3] {
        assert_eq!(scan(&padded(padding, "\u{212A}EY"), "akey"), Scan::Found);
        assert_eq!(scan(&padded(padding, "İX"), "ai"), Scan::Found);
        assert_eq!(scan(&padded(padding, "İX"), "aix"), Scan::Absent);
    }
}
