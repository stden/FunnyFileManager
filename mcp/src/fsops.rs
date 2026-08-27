//! Filesystem side effects that need more care than a single `std::fs` call.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

#[cfg(test)]
mod tests;

/// Remove `path` without ever following a final symlink.
///
/// `remove_dir_all` on a link-to-directory deletes the *target* tree, so every
/// branch below is decided from one `symlink_metadata` call — never from an
/// `is_dir()`/`exists()` probe, which follows the link and re-stats the path.
/// Reading the metadata also keeps dangling links removable.
pub fn remove_entry(path: &Path, recursive: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;

    if meta.is_symlink() {
        // Windows needs `remove_dir` for a link that points at a directory, and
        // that call has to be decided from the metadata already in hand:
        // `path.is_dir()` follows the link — the exact behaviour this function
        // exists to avoid — and re-stats, racing the check two lines up. A
        // dangling directory link has no target to follow and would become
        // undeletable.
        #[cfg(windows)]
        if std::os::windows::fs::FileTypeExt::is_symlink_dir(&meta.file_type()) {
            return fs::remove_dir(path);
        }
        return fs::remove_file(path);
    }

    if meta.is_dir() {
        return if recursive {
            fs::remove_dir_all(path)
        } else {
            fs::remove_dir(path)
        };
    }

    fs::remove_file(path)
}

/// Overwrite an existing regular file with `content` through a temp file plus
/// rename.
///
/// A plain `fs::write` that fails halfway leaves the file truncated; going
/// through a rename makes the replacement all-or-nothing and lets the caller
/// report the failure instead of silently claiming success.
pub fn write_replaced(path: &Path, content: &str) -> io::Result<()> {
    if path.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path has no file name: {}", path.display()),
        ));
    }

    // Only a regular file may be replaced. `symlink_metadata` (never
    // `metadata`) so a symlink is seen as a symlink: renaming over one silently
    // destroys the link, and the content just read came from wherever it
    // pointed — possibly outside the server root.
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "not a regular file, refusing to replace: {}",
                path.display()
            ),
        ));
    }

    // A relative path like "f.txt" has an empty parent, which is the cwd.
    let parent = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };

    // The temp file must be unguessable and freshly created. A fixed sibling
    // name (".<file>.ffm-tmp") written with `fs::write` is neither: anyone who
    // can drop that name into the directory as a symlink gets our content —
    // and, via `set_permissions`, the replaced file's mode — applied to
    // whatever it points at, and then the link renamed over the real file.
    // `NamedTempFile` opens O_EXCL with a random name, so the handle can only
    // be a regular file this process just created. Its `Drop` also unlinks the
    // temp on every failure path below.
    let mut temp = NamedTempFile::new_in(parent)?;

    // Carry the replaced file's mode over *before* the content lands, so a 0600
    // file's replacement never briefly exists at the process umask.
    temp.as_file().set_permissions(meta.permissions())?;
    temp.write_all(content.as_bytes())?;
    temp.persist(path).map(|_| ()).map_err(|e| e.error)
}

/// Set or clear the read-only flag without widening access.
pub fn apply_readonly(path: &Path, readonly: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // `Permissions::set_readonly(false)` ORs in 0o222 — it would hand group
        // and world write access to a 0600 file. Touch the owner bit only.
        let mode = fs::metadata(path)?.permissions().mode();
        let new_mode = if readonly {
            mode & !0o222
        } else {
            mode | 0o200
        };
        fs::set_permissions(path, fs::Permissions::from_mode(new_mode))
    }
    #[cfg(not(unix))]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_readonly(readonly);
        fs::set_permissions(path, perms)
    }
}
