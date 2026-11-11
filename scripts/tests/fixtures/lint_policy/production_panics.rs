//! Negative: production code that panics instead of returning the error.

/// Parses one decimal digit.
///
/// # Panics
///
/// When `text` is not a digit.
#[must_use]
pub fn digit(text: &str) -> u8 {
    text.parse().unwrap()
}

/// Everything after the first byte.
///
/// # Panics
///
/// When `bytes` is empty.
#[must_use]
pub fn rest(bytes: &[u8]) -> &[u8] {
    &bytes[1..]
}

/// Everything after the first character.
///
/// # Panics
///
/// When `text` is empty or starts with a multi-byte character.
#[must_use]
pub fn tail(text: &str) -> &str {
    &text[1..]
}

/// Parses a digit other than zero.
///
/// # Panics
///
/// When `text` is not such a digit.
#[must_use]
pub fn positive(text: &str) -> u8 {
    match text.parse() {
        Ok(0) | Err(_) => panic!("not a positive digit"),
        Ok(value) => value,
    }
}
