//! Path sanitization: every caller-supplied path is confined to one root.
//!
//! Canonicalizing `..` is not traversal prevention on its own — an absolute
//! path resolves to itself, so `/etc/passwd` would sail through. Every path
//! that reaches the filesystem therefore has to pass [`confine`] against the
//! root resolved once at startup.
//!
//! And `confine` is only ever handed a path the filesystem itself resolved: a
//! lexically-normalized path satisfies `starts_with(root)` without any symlink
//! on it having been followed, which would make the guard vacuous.

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
///
/// Still refuses a final component that is a dangling symlink: creating "it"
/// would write through the link to whatever it names.
pub fn sanitize_path_create(root: &Path, raw: &str) -> Result<PathBuf, String> {
    sanitize_path_inner(root, raw, true)
}

/// Resolve a path without following a final symlink: only the parent is
/// canonicalized, the last component is kept verbatim.
///
/// Operations that act on the link itself (delete, stat, readlink) must not
/// resolve it — canonicalizing would hand them the target instead, and a
/// dangling link would not resolve at all.
pub fn sanitize_path_nofollow(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let path = absolute_candidate(root, raw)?;
    let Some(file_name) = path.file_name() else {
        // No final component (e.g. the root itself) — nothing to preserve.
        return confine(root, canonicalize(&path)?);
    };
    let parent = path.parent().unwrap_or(root);
    confine(root, canonicalize(parent)?.join(file_name))
}

fn sanitize_path_inner(root: &Path, raw: &str, allow_new_parents: bool) -> Result<PathBuf, String> {
    let path = absolute_candidate(root, raw)?;
    // A create/write destination must end in an actual entry name. Check the
    // caller's lexical path before `exists()`: on Windows, `missing/..` can be
    // reported as the existing parent and would otherwise bypass the later
    // missing-path validation entirely.
    if allow_new_parents
        && matches!(
            Path::new(raw).components().next_back(),
            None | Some(Component::CurDir | Component::ParentDir)
        )
    {
        return Err(format!("invalid path: {raw}"));
    }

    // A path that resolves end to end is judged on its canonical form, so every
    // symlink along it has already been followed before `confine` sees it.
    if path.exists() {
        return confine(root, canonicalize(&path)?);
    }

    // `exists()` follows links, so anything that still has metadata here is a
    // *dangling* symlink. Handing its literal path back would let the caller's
    // `fs::write` follow it to a target `confine` never inspected — the link may
    // name any file on the machine. Operations that legitimately act on the link
    // itself go through `sanitize_path_nofollow` instead.
    if path.symlink_metadata().is_ok() {
        return Err(format!(
            "refusing to resolve '{}': it is a dangling symlink to an unchecked target",
            path.display()
        ));
    }

    // Nothing is there yet, so the last component still has to be a name —
    // otherwise there is no entry to create.
    if path.file_name().is_none() {
        return Err(format!("invalid path: {raw}"));
    }
    let parent = path.parent().unwrap_or(root);
    if !allow_new_parents && !parent.exists() {
        return Err(format!(
            "parent directory does not exist: {}",
            parent.display()
        ));
    }
    resolve_under_existing_ancestor(root, &normalize_lexically(&path))
}

/// Confine the deepest ancestor of `path` that exists on disk, then re-attach
/// the components below it.
///
/// WHY: `confine` can only judge a path the filesystem has already resolved.
/// Handing it a purely lexical path is no check at all — `starts_with(root)`
/// passes while no symlink on the path was ever followed, so `root/link/new/f`
/// sails through even when `link` points at `/var/log`. Canonicalizing the
/// deepest *existing* ancestor forces every link on the way down through the
/// guard; the components below it do not exist, so nothing there can redirect
/// anywhere. Never confine an unresolved path.
fn resolve_under_existing_ancestor(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let mut tail = Vec::new();
    let mut ancestor = path;
    // `symlink_metadata`, not `exists`: a dangling link must stop the walk so
    // `canonicalize` below fails on it, rather than being stepped over as if
    // that component were simply absent.
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
    let path = Path::new(raw);
    if path.is_absolute() {
        Ok(path.to_path_buf())
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

/// Fold `.` and `..` away textually — for paths that do not exist yet.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}
