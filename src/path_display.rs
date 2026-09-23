//! Purpose: Human-readable path labels without changing the paths used for filesystem I/O.

use std::{borrow::Cow, path::Path};

#[cfg(not(windows))]
pub fn label(path: &Path) -> Cow<'_, str> {
    path.to_string_lossy()
}

#[cfg(windows)]
pub fn label(path: &Path) -> Cow<'_, str> {
    match path.to_string_lossy() {
        Cow::Borrowed(text) => without_verbatim_prefix(text),
        Cow::Owned(text) => Cow::Owned(without_verbatim_prefix(&text).into_owned()),
    }
}

/// Windows' verbatim spelling of a drive or share path in its ordinary form —
/// `\\?\C:\x` is `C:\x`, `\\?\UNC\server\share` is `\\server\share` — which is
/// what people type and other programs write. Any other text comes back as
/// it is, a verbatim volume or device name included.
///
/// Text in, text out, so it runs, and is tested, on every platform.
#[must_use]
pub fn without_verbatim_prefix(text: &str) -> Cow<'_, str> {
    if let Some(share) = text.strip_prefix(r"\\?\UNC\") {
        return Cow::Owned(format!(r"\\{share}"));
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest) if starts_with_drive(rest) => Cow::Borrowed(rest),
        _ => Cow::Borrowed(text),
    }
}

/// Whether `text` starts with a drive: a letter and a colon.
fn starts_with_drive(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|letter| letter.is_ascii_alphabetic())
        && chars.next() == Some(':')
}

#[cfg(test)]
mod tests;
