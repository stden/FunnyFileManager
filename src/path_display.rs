//! Purpose: Human-readable path labels without changing the paths used for filesystem I/O.

use std::{borrow::Cow, path::Path};

#[cfg(not(windows))]
pub fn label(path: &Path) -> Cow<'_, str> {
    path.to_string_lossy()
}

#[cfg(windows)]
pub fn label(path: &Path) -> Cow<'_, str> {
    use std::path::{Component, Prefix};
    let text = path.to_string_lossy();
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(_) => Cow::Owned(text[4..].to_owned()),
            Prefix::VerbatimUNC(_, _) => Cow::Owned(format!("\\\\{}", &text[8..])),
            _ => text,
        },
        _ => text,
    }
}

#[cfg(test)]
mod tests;
