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
    let ordinary = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            // `\\?\C:\…` drops its four-byte `\\?\`.
            Prefix::VerbatimDisk(_) => text.get(4..).map(str::to_owned),
            // `\\?\UNC\server\…` becomes `\\server\…`.
            Prefix::VerbatimUNC(_, _) => text.get(8..).map(|rest| format!("\\\\{rest}")),
            _ => None,
        },
        _ => None,
    };
    ordinary.map_or(text, Cow::Owned)
}

#[cfg(test)]
mod tests;
