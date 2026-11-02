//! Purpose: Tests for decoding what `wsl.exe` printed.

use super::utf16le;
use crate::wsl::encoding::decode_text;

#[test]
fn output_supports_bomless_and_bom_utf16_and_utf8() {
    let names = "Ubuntu\r\nДебиан\r\n";
    let little = utf16le(names);
    let big: Vec<_> = names.encode_utf16().flat_map(u16::to_be_bytes).collect();
    for bytes in [
        names.as_bytes().to_vec(),
        [vec![0xef, 0xbb, 0xbf], names.as_bytes().to_vec()].concat(),
        little.clone(),
        [vec![0xff, 0xfe], little].concat(),
        [vec![0xfe, 0xff], big].concat(),
    ] {
        assert_eq!(decode_text(&bytes, false).unwrap(), names);
    }
}

#[test]
fn malformed_complete_output_is_rejected_but_truncated_display_is_tolerated() {
    for bytes in [
        vec![0xff],
        vec![0xff, 0xfe, 0x41],
        vec![0xff, 0xfe, 0, 0xd8],
    ] {
        assert!(decode_text(&bytes, false).is_err());
    }
    assert_eq!(decode_text(&[b'a', 0xe2, 0x82], true).unwrap(), "a�");
    assert_eq!(
        decode_text(&[0xff, 0xfe, b'a', 0, b'b'], true).unwrap(),
        "a"
    );
}
