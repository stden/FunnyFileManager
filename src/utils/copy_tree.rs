//! Purpose: recursive directory copy: its link and mode rules, entry points, and symlink recreation.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::keep_times::{keep_times, Times};
use super::FileMode;

mod steps;
mod walk;

use walk::{copy_tree, Walk};

/// Recreate a symlink instead of following it.
///
/// `DirEntry::file_type` does not follow links but `fs::copy` does, so treating
/// a link as a file turns a link farm into full copies of its targets, fails
/// outright on broken or directory links, and — in the move fallback — destroys
/// the originals afterwards. The link structure must survive the copy.
///
/// Creating the link fails with [`io::ErrorKind::AlreadyExists`] when `dst` is
/// taken, exactly like the exclusive create in `ops::file::copy_entry`, so the
/// caller's "pick another name and retry" loop keeps working.
pub fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, dst).map_err(|error| super::FailedAt::wrap(dst, error))
    }
    #[cfg(windows)]
    {
        // Windows picks the link flavour at creation time, and creating one may
        // need privileges — best effort, the error is reported like any other.
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(target, dst)
        } else {
            std::os::windows::fs::symlink_file(target, dst)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = target;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symlinks are not supported on this platform",
        ))
    }
}

/// Canonical form of the closest existing ancestor of `path`.
/// A copy destination normally does not exist yet, so it cannot be
/// canonicalised directly.
pub fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find_map(|ancestor| ancestor.canonicalize().ok())
}

/// Whether a symbolic link met during a copy is recreated or followed.
///
/// Far's "Copy contents of symbolic links": off, the link is recreated at the
/// destination; on, what it points at is copied in its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPolicy {
    /// Recreate the link. The default, and the only policy that needs no loop
    /// guard: a tree in which no link is followed cannot contain itself.
    Recreate,
    /// Copy what the link points at.
    Follow,
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    copy_dir_all_with(src, dst, CopyRules::PLAIN)
}

/// How a copy treats what it meets on the way down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyRules {
    pub links: LinkPolicy,
    /// The permission bits each copied file ends up with.
    pub modes: FileMode,
    /// The permission bits each copied folder ends up with. A folder is
    /// created afresh, and its source's mode is restated only once its
    /// contents are in: a read-only folder could not be filled otherwise.
    pub folder_modes: FileMode,
    /// Whether every copied file, folder and link keeps its source's times.
    /// A folder gets them once its contents are in, since writing each entry
    /// into it moves its modification time.
    pub times: Times,
}

impl CopyRules {
    /// Links recreated, every file keeping its source's mode, folders made
    /// like any new folder and nothing keeping its times: what
    /// [`copy_dir_all`] and the quick F5 do.
    pub const PLAIN: Self = Self {
        links: LinkPolicy::Recreate,
        modes: FileMode::Source,
        folder_modes: FileMode::Fresh,
        times: Times::Fresh,
    };

    /// What a move across filesystems copies with: everything the rename it
    /// stands in for would have kept — every file's and folder's mode and
    /// times, and links as links — as `mv` does.
    pub const MOVE: Self = Self {
        links: LinkPolicy::Recreate,
        modes: FileMode::Source,
        folder_modes: FileMode::Source,
        times: Times::Source,
    };
}

/// [`copy_symlink`], and the link's own times when the rules keep them.
///
/// A link whose times could not be set is removed again: the name was
/// claimed by this copy, and callers count on a failed copy leaving nothing.
pub fn copy_link(src: &Path, dst: &Path, rules: CopyRules) -> io::Result<()> {
    copy_symlink(src, dst)?;
    if rules.times == Times::Fresh {
        return Ok(());
    }
    let kept = fs::symlink_metadata(src).and_then(|source| keep_times(dst, &source));
    if kept.is_err() {
        // Best effort: the times' error is the one worth reporting.
        drop(fs::remove_file(dst));
    }
    kept
}

/// [`copy_dir_all`], with the caller's rules about links and modes.
pub fn copy_dir_all_with(src: &Path, dst: &Path, rules: CopyRules) -> io::Result<()> {
    // Defence in depth: the walk below would descend into the copy it is
    // writing and duplicate the tree until the path length gives out. Callers
    // guard too — this keeps a future caller from re-introducing it.
    if let (Some(real_src), Some(real_dst)) = (
        canonical_existing_ancestor(src),
        canonical_existing_ancestor(dst),
    ) {
        if real_dst.starts_with(&real_src) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot copy a directory into itself",
            ));
        }
    }
    // The root counts as entered: `logs -> ..` under the cursor is a link
    // whose own tree holds it again. Only a followed link can build a cycle,
    // so only that policy pays for the canonicalisation.
    let mut walk = match rules.links {
        LinkPolicy::Recreate => Walk::unguarded(),
        LinkPolicy::Follow => Walk::guarded(src, dst),
    };
    copy_tree(src, dst, rules, &mut walk)
}

/// Whether a link met during a transfer is followed rather than recreated.
///
/// Only into a regular file or a directory. A dangling link has nothing to
/// copy and would abort the whole tree — and take the partial copy with it; a
/// FIFO would block the file manager inside `open` until a writer appeared,
/// with no way out; a device node would copy until the disk filled. In each of
/// those the link itself is the only sensible thing to carry over, which is
/// also what the unticked box does.
pub fn follows_link(links: LinkPolicy, path: &Path) -> bool {
    links == LinkPolicy::Follow
        && fs::metadata(path).is_ok_and(|target| target.is_file() || target.is_dir())
}
