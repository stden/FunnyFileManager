//! Purpose: The file a save lands on — symlinks resolved, dangling links refused — and whether
//! an atomic rename would keep its owner and its other names.

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

/// A symbolic link that leads nowhere: its target does not exist, the chain
/// loops, or a directory on the way cannot be entered. Carries where the link
/// points, which is what the user has to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingLink(pub PathBuf);

impl fmt::Display for DanglingLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the link points at {}, which does not exist",
            self.0.display()
        )
    }
}

impl DanglingLink {
    /// The refusal as the error a save reports.
    pub fn into_io_error(self) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, self.to_string())
    }
}

/// The file a save must actually land on.
///
/// Persisting the temp file is `rename(2)`, which replaces the *name* it is
/// given instead of writing through it. Handed `~/.vimrc -> ~/dotfiles/vimrc`,
/// the rename would drop a fresh regular file over the link and leave the real
/// file — the one under version control — holding the old text. Resolving
/// first puts both the temp file and the rename on the file the user meant.
///
/// A path with nothing behind it yet (Shift+F4, a first save) is used as
/// given. A link that cannot be resolved is refused: writing to it would turn
/// the link into a regular file, which is the damage this function exists to
/// prevent.
pub fn real_target(path: &Path) -> Result<PathBuf, DanglingLink> {
    match fs::canonicalize(path) {
        Ok(real) => Ok(real),
        Err(_) => fs::read_link(path).map_or_else(
            |_| Ok(path.to_path_buf()),
            |target| Err(DanglingLink(target)),
        ),
    }
}

/// Whether this user owns the file. Unknown counts as owned: the atomic path
/// is the better one, and a file that is not there yet has no owner to keep.
#[cfg(unix)]
pub fn owned_by_us(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let us = crate::platform::effective_uid();
    fs::metadata(path).map_or(true, |info| info.uid() == us)
}

/// Windows has no cheap owner comparison here, and its ACLs survive a rename
/// differently; the atomic path stays.
#[cfg(not(unix))]
pub const fn owned_by_us(_path: &Path) -> bool {
    true
}

/// Whether the file has more than one name (a hard link).
///
/// A rename detaches only the name it replaces, so an atomic save would leave
/// every other name pointing at the old inode — the same silent divergence a
/// symlink suffers, and one `canonicalize` cannot resolve because hard links
/// have no "real" name.
#[cfg(unix)]
pub fn has_other_names(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    fs::metadata(path).is_ok_and(|metadata| metadata.nlink() > 1)
}

/// Hard links are not reachable through the panel on non-Unix targets, so the
/// atomic path is always taken there.
#[cfg(not(unix))]
pub const fn has_other_names(_path: &Path) -> bool {
    false
}
