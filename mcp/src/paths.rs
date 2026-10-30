//! Path sanitization: every caller-supplied path is confined to one root.
//!
//! Canonicalizing `..` is not traversal prevention on its own — an absolute
//! path resolves to itself, so `/etc/passwd` would sail through. Every path
//! that reaches the filesystem therefore has to pass [`confine`] against the
//! root resolved once at startup.

use std::path::{Component, Path, PathBuf};

#[cfg(test)]
mod tests;

/// Environment variable naming the only directory tree the server may touch.
pub const ROOT_ENV: &str = "FFM_MCP_ROOT";

/// Resolve the allowed root once at startup: `$FFM_MCP_ROOT`, else the cwd.
pub fn resolve_root() -> Result<PathBuf, String> {
    let raw = match std::env::var_os(ROOT_ENV) {
        Some(value) => PathBuf::from(value),
        None => std::env::current_dir()
            .map_err(|e| format!("cannot determine current directory: {e}"))?,
    };
    raw.canonicalize()
        .map_err(|e| format!("cannot resolve {ROOT_ENV} '{}': {e}", raw.display()))
}

/// Resolve an existing path (symlinks followed) inside `root`.
pub fn sanitize_path(root: &Path, raw: &str) -> Result<PathBuf, String> {
    sanitize_path_inner(root, raw, false)
}

/// Like [`sanitize_path`], but tolerates a parent that does not exist yet.
pub fn sanitize_path_create(root: &Path, raw: &str) -> Result<PathBuf, String> {
    sanitize_path_inner(root, raw, true)
}

/// Resolve a path without following a final symlink: only the parent is
/// canonicalized, the last component is kept verbatim.
pub fn sanitize_path_nofollow(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let path = absolute_candidate(root, raw)?;
    let Some(file_name) = path.file_name().filter(|n| *n != "." && *n != "..") else {
        return confine(root, canonicalize(&path)?);
    };
    let parent = path.parent().unwrap_or(root);
    confine(root, canonicalize(parent)?.join(file_name))
}

fn sanitize_path_inner(root: &Path, raw: &str, allow_new_parents: bool) -> Result<PathBuf, String> {
    let path = absolute_candidate(root, raw)?;
    if allow_new_parents && ends_with_dot_component(raw) {
        return Err(format!("invalid path: {raw}"));
    }
    if path.exists() {
        return confine(root, canonicalize(&path)?);
    }
    if path.symlink_metadata().is_ok() {
        return Err(format!(
            "refusing to resolve '{}': it is a dangling symlink to an unchecked target",
            path.display()
        ));
    }
    if path.file_name().is_none() || ends_with_dot_component(raw) {
        return Err(format!("invalid path: {raw}"));
    }
    let normalized = resolve_dot_dot(&path)?;
    let parent = normalized.parent().unwrap_or(root);
    if !allow_new_parents && !parent.exists() {
        return Err(format!(
            "parent directory does not exist: {}",
            parent.display()
        ));
    }
    resolve_under_existing_ancestor(root, &normalized)
}

fn ends_with_dot_component(raw: &str) -> bool {
    let trimmed = raw.trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    trimmed.is_empty() || matches!(last, "." | "..")
}

/// Confine the deepest ancestor of `path` that exists on disk, then re-attach
/// the components below it.
fn resolve_under_existing_ancestor(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let mut tail = Vec::new();
    let mut ancestor = path;
    while ancestor.symlink_metadata().is_err() {
        let (Some(name), Some(parent)) = (ancestor.file_name(), ancestor.parent()) else {
            return Err(format!(
                "cannot resolve path '{}': it has no existing ancestor directory",
                path.display()
            ));
        };
        tail.push(name);
        ancestor = parent;
    }
    let mut resolved = confine(root, canonicalize(ancestor)?)?;
    resolved.extend(tail.iter().rev());
    Ok(resolved)
}

/// Validate `raw` and turn it into an absolute path. Relative paths are
/// interpreted against the allowed root, which defaults to the cwd.
fn absolute_candidate(root: &Path, raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err("path must not be empty".to_owned());
    }
    if raw.contains('\0') {
        return Err("path contains null byte".to_owned());
    }
    #[cfg(windows)]
    let cleaned = raw
        .strip_prefix(r"\\?\UNC\")
        .map(|unc| format!(r"\\{unc}"))
        .or_else(|| raw.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or_else(|| raw.replace('/', "\\"));
    #[cfg(not(windows))]
    let cleaned = raw;

    let path = Path::new(&cleaned);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else if matches!(path.components().next(), Some(Component::Prefix(_))) {
        Err(format!("invalid relative path: {raw}"))
    } else {
        Ok(root.join(path))
    }
}

fn canonicalize(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("cannot resolve path '{}': {e}", path.display()))
}

/// The single gate every resolved path passes through.
fn confine(root: &Path, resolved: PathBuf) -> Result<PathBuf, String> {
    if resolved.starts_with(root) {
        return Ok(resolved);
    }
    Err(format!(
        "path '{}' is outside the allowed root '{}' (set {ROOT_ENV} to change it)",
        resolved.display(),
        root.display()
    ))
}

/// Fold `.` and `..`, resolving any existing symlink before stepping into `..`.
fn resolve_dot_dot(path: &Path) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        if is_cur_dir(&component) {
            continue;
        }
        if is_parent_dir(&component) {
            if out.symlink_metadata().is_ok() {
                out = canonicalize(&out)?;
            }
            out.pop();
        } else {
            out.push(component);
        }
    }
    Ok(out)
}

/// Fold `.` and `..` away textually — for paths that do not exist yet.
#[cfg(test)]
pub fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        if is_cur_dir(&component) {
            continue;
        }
        if is_parent_dir(&component) {
            out.pop();
        } else {
            out.push(component);
        }
    }
    out
}

fn is_cur_dir(component: &Component<'_>) -> bool {
    matches!(component, Component::CurDir)
        || matches!(component, Component::Normal(part) if *part == ".")
}

fn is_parent_dir(component: &Component<'_>) -> bool {
    matches!(component, Component::ParentDir)
        || matches!(component, Component::Normal(part) if *part == "..")
}
