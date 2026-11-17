//! Shell quoting for file paths used by menu command templates.

use std::path::Path;

#[cfg(unix)]
pub(super) fn quote(path: &Path) -> String {
    let clean = path.display().to_string().replace(['\n', '\r', '\0'], "");
    format!("'{}'", clean.replace('\'', "'\\''"))
}

#[cfg(windows)]
pub(super) fn quote(path: &Path) -> String {
    let clean = path
        .display()
        .to_string()
        .replace(['\n', '\r', '\0'], "")
        .replace('"', "\"\"");
    format!("\"{clean}\"")
}

#[cfg(test)]
mod tests;
