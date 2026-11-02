//! Purpose: Unit tests for WSL output text decoding across UTF-8 (including embedded NULs) and UTF-16.

use super::decode_text;

#[test]
fn utf8_with_embedded_or_trailing_nul_is_not_misread_as_utf16le() {
    assert_eq!(decode_text(b"a\0", false).expect("short utf8+nul"), "a");
    assert_eq!(
        decode_text(b"hello\0world\n", false).expect("nul-separated utf8"),
        "hello\0world\n"
    );
    assert_eq!(
        decode_text(b"odd\0len", false).expect("odd-length utf8 with nul"),
        "odd\0len"
    );
}

#[test]
fn bomless_utf16le_from_wsl_still_decodes_as_utf16le() {
    let utf16: Vec<u8> = "Ubuntu\r\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(decode_text(&utf16, false).expect("utf16le"), "Ubuntu\r\n");
}
