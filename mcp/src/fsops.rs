//! Filesystem side effects that need more care than a single `std::fs` call.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

mod copy;
mod moving;
mod remove_moved;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use copy::open_folder;
pub use copy::{copy_entry, Purpose};
#[cfg(test)]
pub use moving::FORCE_RENAME_FAILURE;
pub use moving::{move_entry, Moved};
#[cfg(test)]
pub use remove_moved::{remove_moved, FORCE_REMOVAL_REFUSAL};

/// Remove `path` without ever following a final symlink.
///
/// `remove_dir_all` on a link-to-directory deletes the *target* tree, so every
/// branch below is decided from one `symlink_metadata` call — never from an
/// `is_dir()`/`exists()` probe, which follows the link and re-stats the path.
/// Reading the metadata also keeps dangling links removable.
pub fn remove_entry(path: &Path, recursive: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;

    if meta.is_symlink() {
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
pub fn write_replaced(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    let meta = ensure_replaceable_regular_file(path)?;

    // A relative path like "f.txt" has an empty parent, which is the cwd.
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(parent)?;

    // Carry the replaced file's mode over *before* the content lands, so a 0600
    // file's replacement never briefly exists at the process umask.
    temp.as_file().set_permissions(meta.permissions())?;
    temp.write_all(content.as_ref())?;
    temp.persist(path).map(|_| ()).map_err(|e| e.error)
}

fn ensure_replaceable_regular_file(path: &Path) -> io::Result<fs::Metadata> {
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

    // On Windows a read-only destination refuses `MoveFileExW` and a read-only
    // temp file refuses cleanup on drop; refuse up front before creating a temp.
    #[cfg(not(unix))]
    if meta.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("file is marked read-only: {}", path.display()),
        ));
    }
    Ok(meta)
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
