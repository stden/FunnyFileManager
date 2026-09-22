//! Shell quoting for file paths used by menu command templates.

use std::path::Path;

#[cfg(unix)]
pub(super) fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[cfg(windows)]
pub(super) fn quote(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

#[cfg(test)]
mod tests;
