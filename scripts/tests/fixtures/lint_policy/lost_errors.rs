//! Negative: errors dropped without a word.

use std::path::Path;

/// Parses one decimal digit.
///
/// # Errors
///
/// When `text` is not a digit; why is thrown away.
pub fn digit(text: &str) -> Result<u8, &'static str> {
    text.parse().map_err(|_| "not a digit")
}

/// A failed removal is silently ignored.
pub fn remove(path: &Path) {
    std::fs::remove_file(path).ok();
}

/// A parse failure is silently ignored.
pub fn check(text: &str) {
    let _ = text.parse::<u8>();
}
