//! Purpose: Destination paths built from a caller-supplied file name, which may
//! add exactly one component to a directory already confined to the root.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
mod tests;

/// Join a caller-supplied name onto a directory that is already confined to
/// the server root — the only sanctioned way to build a path from an argument
/// the sandbox never inspected.
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
    const FAST_PROBES: u32 = 4;
    let dest = join_child(base, file_name)?;
    if dest.symlink_metadata().is_err() {
        return Ok(dest);
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str());

    for counter in 1..=FAST_PROBES {
        let candidate = base.join(numbered_name(stem, extension, counter));
        if candidate.symlink_metadata().is_err() {
            return Ok(candidate);
        }
    }
    probe_with_snapshot(
        base,
        file_name,
        stem,
        extension,
        FAST_PROBES + 1,
        MAX_ATTEMPTS,
    )
}

fn numbered_name(stem: &str, extension: Option<&str>, counter: u32) -> String {
    match extension {
        Some(ext) if !ext.is_empty() => format!("{stem}_{counter}.{ext}"),
        _ => format!("{stem}_{counter}"),
    }
}

fn probe_with_snapshot(
    base: &Path,
    file_name: &str,
    stem: &str,
    extension: Option<&str>,
    start: u32,
    max_attempts: u32,
) -> Result<PathBuf, String> {
    let occupied = crate::fsops::occupied_names(base);
    for counter in start..=max_attempts {
        let new_name = numbered_name(stem, extension, counter);
        if !occupied.contains(OsStr::new(&new_name)) {
            return Ok(base.join(new_name));
        }
    }
    Err(format!(
        "could not find unique name for '{file_name}' after {max_attempts} attempts"
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
