use super::*;

#[test]
fn cyrillic_code_pages_match_independent_byte_vectors() {
    for (encoding, bytes) in [
        (
            TextEncoding::Windows1251,
            &[0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2][..],
        ),
        (
            TextEncoding::Oem866,
            &[0x8f, 0xe0, 0xa8, 0xa2, 0xa5, 0xe2][..],
        ),
    ] {
        assert_eq!(encoding.decode(bytes).expect("decode"), "Привет");
        assert_eq!(encoding.encode("Привет").expect("encode"), bytes);
    }
}

#[test]
fn unicode_encodings_preserve_bom_byte_order_and_surrogate_pairs() {
    for (encoding, bytes) in [
        (
            TextEncoding::Utf8Bom,
            &[0xef, 0xbb, 0xbf, 0xf0, 0x9f, 0x99, 0x82][..],
        ),
        (
            TextEncoding::Utf16Le,
            &[0xff, 0xfe, 0x3d, 0xd8, 0x42, 0xde][..],
        ),
        (
            TextEncoding::Utf16Be,
            &[0xfe, 0xff, 0xd8, 0x3d, 0xde, 0x42][..],
        ),
    ] {
        assert_eq!(encoding.decode(bytes).expect("decode"), "🙂");
        assert_eq!(encoding.encode("🙂").expect("encode"), bytes);
    }
}

#[test]
fn all_codecs_round_trip_representable_text() {
    for encoding in TextEncoding::ALL {
        let text = if encoding == TextEncoding::Windows1252 {
            "café\r\n"
        } else {
            "Привет\r\n"
        };
        let bytes = encoding.encode(text).expect("encode");
        assert_eq!(encoding.decode(&bytes).expect("decode"), text);
        assert_ne!(encoding.label().len(), 0);
    }
}

#[test]
fn detection_recognizes_unicode_boms_and_legacy_fallback() {
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf8Bom,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Windows1251,
    ] {
        assert_eq!(
            TextEncoding::detect(&encoding.encode("Привет").expect("encode")),
            encoding
        );
    }
}

#[test]
fn unsupported_characters_and_malformed_unicode_are_errors() {
    for encoding in [
        TextEncoding::Windows1251,
        TextEncoding::Oem866,
        TextEncoding::Windows1252,
    ] {
        assert!(encoding.encode("🙂").is_err());
    }
    assert!(TextEncoding::Utf8.decode(&[0xff]).is_err());
    assert!(TextEncoding::Utf16Le.decode(&[0xff, 0xfe, 1]).is_err());
    assert!(TextEncoding::Utf16Be
        .decode(&[0xfe, 0xff, 0xd8, 0])
        .is_err());
}

#[test]
fn a_lossy_decode_replaces_what_the_strict_one_refuses_and_drops_the_bom() {
    let damaged = b"\xff\xfeO\x00K\x00!";
    assert!(TextEncoding::Utf16Le.decode(damaged).is_err());
    assert_eq!(TextEncoding::Utf16Le.decode_lossy(damaged), "OK\u{fffd}");
    assert_eq!(TextEncoding::Utf8Bom.decode_lossy(b"\xef\xbb\xbfhi"), "hi");
    assert!(matches!(
        TextEncoding::Utf8.decode_lossy_cow(b"hello"),
        std::borrow::Cow::Borrowed("hello")
    ));
}

#[test]
fn the_oem_ansi_toggle_swaps_the_two_cyrillic_pages_and_sends_others_to_oem() {
    assert_eq!(
        TextEncoding::Windows1251.oem_ansi_toggle(),
        TextEncoding::Oem866
    );
    assert_eq!(
        TextEncoding::Oem866.oem_ansi_toggle(),
        TextEncoding::Windows1251
    );
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf8Bom,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Windows1252,
    ] {
        assert_eq!(encoding.oem_ansi_toggle(), TextEncoding::Oem866);
    }
}

#[test]
fn a_nul_byte_is_binary_unless_a_utf16_byte_order_mark_explains_it() {
    assert!(is_binary(b"MZ\x00\x01"));
    assert!(!is_binary(b"plain text"));
    assert!(!is_binary(b"\xff\xfeH\x00i\x00"));
    assert!(!is_binary(b"\xfe\xff\x00H\x00i"));
    // UTF-16 without a mark is indistinguishable from a binary here.
    assert!(is_binary(b"H\x00i\x00"));
    // Behind the mark, a NUL character is still a binary's.
    assert!(is_binary(b"\xff\xfeH\x00\x00\x00i\x00"));
    assert!(!is_binary(b"\xff\xfe\x00\x01"), "U+0100 is text");
}
