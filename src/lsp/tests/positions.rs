//! Purpose: Unicode position conversion and portable file URI regressions.

use super::super::{positions::*, uri};
use tempfile::TempDir;

#[test]
fn character_and_byte_positions_round_trip_ascii_cyrillic_emoji_and_lines() {
    for text in ["hello", "Привет", "😀🚀", "aЯ😀z\nб🚀x\n"] {
        for byte in text
            .char_indices()
            .map(|(byte, _)| byte)
            .chain([text.len()])
        {
            let position = at_byte(text, byte).unwrap();
            assert_eq!(
                byte_offset(text, position),
                Some(byte),
                "{text:?} at {byte}"
            );
            let line_index = usize::try_from(position.line).unwrap();
            let line = text.split('\n').nth(line_index).unwrap();
            let chars = to_chars(line, position.character).unwrap();
            assert_eq!(from_chars(line_index, line, chars), Some(position));
        }
    }
}

#[test]
fn surrogate_interiors_and_invalid_ranges_are_rejected() {
    assert_eq!(from_chars(0, "A😀Б", 2).unwrap().character, 3);
    assert_eq!(to_chars("A😀Б", 2), None);
    assert_eq!(from_chars(0, "x", 2), None);
    assert_eq!(at_byte("😀", 1), None);
    assert_eq!(
        byte_offset(
            "x",
            Position {
                line: 1,
                character: 0
            }
        ),
        None
    );
    assert_eq!(
        byte_offset(
            "x",
            Position {
                line: 0,
                character: 2
            }
        ),
        None
    );
}

#[test]
fn reversed_ranges_are_rejected_and_crlf_offsets_are_correct() {
    assert_eq!(
        at_byte("a\r\nb", 2),
        None,
        "CRLF interiors have no LSP position"
    );
    assert_eq!(
        at_byte("a\r\nb", 1),
        Some(Position {
            line: 0,
            character: 1
        })
    );
    let reversed = Range {
        start: Position {
            line: 0,
            character: 1,
        },
        end: Position::default(),
    };
    assert_eq!(byte_range("x", reversed), None);
    assert_eq!(
        byte_offset(
            "a\r\nб",
            Position {
                line: 1,
                character: 1
            }
        ),
        Some(5)
    );
}

#[test]
fn file_uris_round_trip_spaces_unicode_and_reserved_characters() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("a # %00 Я😀.rs");
    let uri = uri::from_path(&path).unwrap();
    assert!(uri.contains("%20") && uri.contains("%23"));
    assert_eq!(uri::to_path(&uri), Some(path));
    assert!(uri::same(&uri, &uri));
    assert!(!uri::same(&uri, "https://example.com/file"));
    for invalid in [
        "not a URI",
        "https://example.com/a.rs",
        "file:///tmp/x?query",
        "file:///tmp/x#part",
        "file:///tmp/%00",
    ] {
        assert!(uri::to_path(invalid).is_none(), "{invalid}");
    }
    assert!(uri::from_path(std::path::Path::new("relative.rs")).is_none());
}

#[cfg(windows)]
#[test]
fn windows_drive_and_unc_uris_are_native_paths() {
    for (uri, path) in [
        ("file:///C:/work/a%20b.rs", r"C:\work\a b.rs"),
        ("file://server/share/a.rs", r"\\server\share\a.rs"),
    ] {
        assert_eq!(uri::to_path(uri), Some(path.into()));
        assert_eq!(
            uri::from_path(std::path::Path::new(path)).as_deref(),
            Some(uri)
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_file_identity_accepts_drive_case_and_mixed_separators() {
    use std::path::Path;
    assert!(uri::same_path(
        Path::new(r"C:\work\src/main.rs"),
        Path::new(r"c:\work\src\main.rs")
    ));
    assert!(uri::same(
        "file:///C:/work/a%20b.rs",
        "file:///c:/work/a%20b.rs"
    ));
    assert!(!uri::same("file:///C:/work/a.rs", "file:///C:/work/b.rs"));
    assert!(!uri::same(
        "https://example.com/file",
        "https://example.com/file"
    ));
}

#[test]
fn utf16_column_byte_maps_ascii_and_surrogate_pairs_directly() {
    assert_eq!(column_byte("hello", 3), Some(3));
    assert_eq!(column_byte("A😀Б", 3), Some(5));
    assert_eq!(column_byte("A😀Б", 2), None);
}
