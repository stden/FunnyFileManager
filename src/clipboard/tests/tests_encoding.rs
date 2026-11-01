//! Unit tests: the base64 the OSC 52 route sends, and redaction of an empty
//! secret.

use super::*;

#[test]
fn base64_matches_rfc4648_vectors() {
    let vectors = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];
    for (input, expected) in vectors {
        assert_eq!(
            base64_encode(input.as_bytes()),
            expected,
            "base64({input:?})"
        );
    }
}

#[test]
fn base64_encodes_high_bytes_without_sign_or_truncation_errors() {
    // 0xFF 0xFE 0xFD exercises every high bit; "пароль" is multi-byte UTF-8,
    // which is what a real password field can hold.
    assert_eq!(base64_encode(&[0xFF, 0xFE, 0xFD]), "//79");
    assert_eq!(base64_encode("пароль".as_bytes()), "0L/QsNGA0L7Qu9GM");
}

#[test]
fn redact_with_an_empty_secret_returns_the_message_untouched() {
    // `str::replace("", ..)` would match between every character; the guard
    // must hand the message back as-is instead.
    assert_eq!(redact("native failed: x", ""), "native failed: x");
}
