//! Purpose: Replace a file's contents without changing who may write it — the
//! writability check, the symlink and hard-link rules, and the atomic
//! temp-file-and-rename that the built-in editor, the `KeePass` writer and the
//! settings writer share.
//!
//! One module because every writer replaces a file by renaming a fresh temp
//! file over it, and that rename, left to itself, ignores the file's own
//! permissions and hands it a new mode: the editor had learned both lessons,
//! the vault writer had not.

use std::{
    fmt, fs,
    io::{self, Write},
    path::Path,
};

mod target;

pub use target::{has_other_names, owned_by_us, real_target, DanglingLink};

/// Refuse a write the filesystem would otherwise let through for the wrong
/// reason.
///
/// Persisting a temp file is `rename(2)`, which replaces a directory
/// *entry*: the kernel checks write permission on the parent directory and
/// never looks at the file. A root-owned 0644 file sitting in the user's own
/// directory was therefore replaced in silence — new inode, new owner, ACLs
/// and xattrs gone — while the editor reported "Saved", and a 0444 `KeePass`
/// vault came back 0600. An in-place write fails on its own, because
/// `fs::write` really opens the file, but the check sits ahead of both so
/// that one refusal has one message.
pub fn ensure_writable(target: &Path) -> io::Result<()> {
    match refusal(target) {
        None => Ok(()),
        Some(refusal) => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{}: {refusal}", target.display()),
        )),
    }
}

/// Why a write is refused before it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteRefusal {
    /// The kernel denies this process write access to the file.
    #[cfg(unix)]
    PermissionDenied,
    /// The whole filesystem is mounted read-only. "Permission denied" would
    /// send the user to `ls -l` and a chmod that cannot help.
    #[cfg(unix)]
    ReadOnlyFileSystem,
    /// The file's read-only attribute is set.
    #[cfg(not(unix))]
    MarkedReadOnly,
}

impl fmt::Display for WriteRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            #[cfg(unix)]
            Self::PermissionDenied => "permission denied",
            #[cfg(unix)]
            Self::ReadOnlyFileSystem => "read-only file system",
            #[cfg(not(unix))]
            Self::MarkedReadOnly => "the file is marked read-only",
        })
    }
}

/// Whether *this* process may write the file's contents.
///
/// `metadata().permissions().readonly()` reads the owner-write bit alone and
/// says nothing about who is asking, so a root-owned 0644 file looks writable
/// to every user on the machine. Only the kernel knows the real answer, and
/// only the kernel accounts for group membership, POSIX ACLs, a read-only
/// mount and root's `CAP_DAC_OVERRIDE` — so root is not blocked, which is
/// correct. The question is asked with the *effective* ids, which are the
/// ones the write will use.
///
/// A path with nothing behind it yet (Shift+F4) answers `ENOENT`, and an
/// unexpected errno — an interior NUL included — means the question could not
/// be put at all: both fall open, leaving the write that follows to report the
/// real failure rather than this guard inventing one.
#[cfg(unix)]
fn refusal(path: &Path) -> Option<WriteRefusal> {
    let Err(error) = crate::platform::check_write_access(path) else {
        return None;
    };
    match error.raw_os_error() {
        Some(libc::EROFS) => Some(WriteRefusal::ReadOnlyFileSystem),
        Some(libc::EACCES | libc::EPERM) => Some(WriteRefusal::PermissionDenied),
        _ => None,
    }
}

/// Windows has one writability bit per file and `readonly()` reads exactly
/// it. A file that is not there yet is the directory's business, as on Unix.
#[cfg(not(unix))]
fn refusal(path: &Path) -> Option<WriteRefusal> {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.permissions().readonly())
        .then_some(WriteRefusal::MarkedReadOnly)
}

/// A temp file in `target`'s own directory — the rename must not cross
/// filesystems, or it stops being atomic — or why the directory refused one.
///
/// The refusal names the directory: the temp file's random name is nothing
/// the user chose or could act on, and it used to be all the message said.
pub fn temp_beside(target: &Path) -> io::Result<tempfile::NamedTempFile> {
    let dir = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    tempfile::NamedTempFile::new_in(dir).map_err(|err| {
        let kind = err.kind();
        io::Error::new(
            kind,
            format!("cannot create a file in {}: {kind}", dir.display()),
        )
    })
}

/// Give `temp` the target's permissions, fill it, flush it to disk and
/// rename it over `target`.
pub fn persist_over(
    mut temp: tempfile::NamedTempFile,
    target: &Path,
    content: &[u8],
) -> io::Result<()> {
    fill(&mut temp, target, content)?;
    temp.persist(target).map_err(|err| err.error)?;
    Ok(())
}

/// Everything [`persist_over`] does but the rename: the target's
/// permissions, the content, the flush. A writer that must look at the
/// target once more before replacing it renames afterwards itself.
pub fn fill(temp: &mut tempfile::NamedTempFile, target: &Path, content: &[u8]) -> io::Result<()> {
    // Temp files are created 0600 and the rename carries the mode onto the
    // target, so the original's permissions have to be copied over first.
    if let Ok(metadata) = fs::metadata(target) {
        temp.as_file().set_permissions(metadata.permissions())?;
    }
    temp.write_all(content)?;
    temp.as_file().sync_all()
}

#[cfg(test)]
mod tests;
