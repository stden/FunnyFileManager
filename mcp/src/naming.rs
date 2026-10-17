//! Purpose: Destination paths built from a caller-supplied file name, which may
//! add exactly one component to a directory already confined to the root.

use std::path::{Component, Path, PathBuf};

/// Join a caller-supplied name onto a directory that is already confined to
/// the server root — the only sanctioned way to build a path from an argument
/// the sandbox never inspected.
///
/// WHY: `Path::join` is not a string append. An absolute operand *replaces* the
/// base outright, and `..` components walk back out of it, so joining a raw
/// name onto a confined directory throws the confinement away:
/// `join(root, "../../../../etc/passwd")` lands on `/etc/passwd`, and a caller
/// who cannot read that file can still learn whether it exists from whatever
/// the tool reports back. Exactly one ordinary component may be joined.
pub fn join_child(base: &Path, name: &str) -> Result<PathBuf, String> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(single)), None) => Ok(base.join(single)),
        _ => Err(format!(
            "'{name}' is not a plain file name: exactly one path component is allowed"
        )),
    }
}

/// Generate a unique file path to avoid overwriting existing files.
/// If `test.txt` already exists, returns `test_1.txt`, `test_2.txt`, etc.
pub fn unique_destination(base: &Path, file_name: &str) -> Result<PathBuf, String> {
    const MAX_ATTEMPTS: u32 = 9999;
    // The counter suffixes below are derived from this one validated component,
    // so `{stem}_{n}[.{ext}]` can never grow a separator or a `..` of its own.
    let mut dest = join_child(base, file_name)?;
    if !dest.exists() {
        return Ok(dest);
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str());

    for counter in 1..=MAX_ATTEMPTS {
        let new_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem}_{counter}.{ext}"),
            _ => format!("{stem}_{counter}"),
        };
        dest = base.join(new_name);
        if !dest.exists() {
            return Ok(dest);
        }
    }
    Err(format!(
        "could not find unique name for '{file_name}' after {MAX_ATTEMPTS} attempts"
    ))
}

/// The final component of a resolved path, as the batch tools report it and as
/// they name the copy at the destination.
pub fn entry_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
