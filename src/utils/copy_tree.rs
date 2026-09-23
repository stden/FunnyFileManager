//! Purpose: recursive directory copy: each filesystem step with its forced-failure seam, link recreation, and the loop-guarded walk.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(any(test, tarpaulin))]
use super::{
    force_lock, COPY_DIR_ALL_FAIL_COPY_FILE, COPY_DIR_ALL_FAIL_CREATE, COPY_DIR_ALL_FAIL_ENTRY,
    COPY_DIR_ALL_FAIL_FILE_TYPE, COPY_DIR_ALL_FAIL_RECURSE, FORCE_COPY_DIR_ALL_STEP,
};

fn create_dir_all_checked(dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_CREATE {
            return Err(io::Error::other("forced create_dir_all failure"));
        }
    }
    fs::create_dir_all(dst)
}

#[allow(clippy::missing_const_for_fn)] // Cannot be const when #[cfg(test)] block is present
fn read_dir_entry(entry: io::Result<fs::DirEntry>) -> io::Result<fs::DirEntry> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_ENTRY {
            return Err(io::Error::other("forced read_dir entry failure"));
        }
    }
    entry
}

fn entry_file_type(entry: &fs::DirEntry) -> io::Result<fs::FileType> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_FILE_TYPE {
            return Err(io::Error::other("forced file_type failure"));
        }
    }
    entry.file_type()
}

fn copy_file_checked(src: &Path, dst: &Path) -> io::Result<u64> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_COPY_FILE {
            return Err(io::Error::other("forced copy failure"));
        }
    }
    // `fs::copy` opens the source before it looks at what the source is, and
    // `open` on a FIFO waits for a writer that may never come — with the event
    // loop stopped and no key able to reach it. A device node would copy until
    // the disk filled instead. Neither has contents a file manager can carry
    // over, so both are reported rather than attempted.
    if !fs::metadata(src)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{}: not a regular file", src.display()),
        ));
    }
    fs::copy(src, dst)
}

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
        std::os::unix::fs::symlink(target, dst)
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
    copy_dir_all_with(src, dst, LinkPolicy::Recreate)
}

/// [`copy_dir_all`], with the caller's choice about links.
pub fn copy_dir_all_with(src: &Path, dst: &Path, links: LinkPolicy) -> io::Result<()> {
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
    let mut walk = match links {
        LinkPolicy::Recreate => Walk::unguarded(),
        LinkPolicy::Follow => Walk::guarded(src, dst),
    };
    copy_tree(src, dst, links, &mut walk)
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

/// What the walk must not enter twice, or at all.
///
/// Empty under [`LinkPolicy::Recreate`], where no link is followed and a real
/// directory tree cannot contain itself.
struct Walk {
    /// The canonical path of every directory on the way down.
    entered: Vec<PathBuf>,
    /// The copy's own output. A link that reaches into it would have the walk
    /// copying what it is writing — every level a fresh path, so the `entered`
    /// stack alone never notices.
    output: Option<PathBuf>,
}

impl Walk {
    const fn unguarded() -> Self {
        Self {
            entered: Vec::new(),
            output: None,
        }
    }

    fn guarded(src: &Path, dst: &Path) -> Self {
        Self {
            entered: src.canonicalize().ok().into_iter().collect(),
            output: dst.canonicalize().ok(),
        }
    }

    /// Why this directory must not be walked, if it must not be.
    fn refusal(&self, real: &Path, src: &Path) -> Option<io::Error> {
        if self
            .output
            .as_ref()
            .is_some_and(|out| real.starts_with(out))
        {
            return Some(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} leads into the copy itself", src.display()),
            ));
        }
        self.entered
            .iter()
            .any(|open| open.starts_with(real))
            .then(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("symbolic link loop at {}", src.display()),
                )
            })
    }
}

fn copy_tree(src: &Path, dst: &Path, links: LinkPolicy, walk: &mut Walk) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_RECURSE
            && src
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == ".ffm_force_copy_dir_error")
        {
            return Err(io::Error::other("forced recursive copy failure"));
        }
    }

    create_dir_all_checked(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = read_dir_entry(entry)?;
        let file_type = entry_file_type(&entry)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_symlink() && !follows_link(links, &src_path) {
            copy_symlink(&src_path, &dst_path)?;
        } else if src_path.is_dir() {
            // `is_dir` follows the link, which is the right question to ask
            // once the policy says to follow it.
            descend(&src_path, &dst_path, links, walk)?;
        } else {
            copy_file_checked(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Copy a subdirectory, refusing one that leads back into the walk.
///
/// Only [`LinkPolicy::Follow`] can build a cycle, and it can build two.
/// `ln -s .. loop` walks up into a directory the walk is already inside,
/// which the `entered` stack catches; a link into the destination has the
/// walk copying its own output, every level a path it has never seen, which
/// only the recorded output catches. Either is refused with an error the
/// batch reports rather than recursing until the path length gives out. The
/// stack entry is popped on the way back up, so two links to the same
/// directory side by side are duplicated work, not a loop.
fn descend(src: &Path, dst: &Path, links: LinkPolicy, walk: &mut Walk) -> io::Result<()> {
    if links == LinkPolicy::Recreate {
        // Nothing to guard against, and nothing to pay for it: a link never
        // reaches this function under `Recreate`, so every descent is into a
        // real subdirectory and a real directory tree cannot contain itself.
        return copy_tree(src, dst, links, walk);
    }
    let real = src.canonicalize()?;
    if let Some(refusal) = walk.refusal(&real, src) {
        return Err(refusal);
    }
    walk.entered.push(real);
    let result = copy_tree(src, dst, links, walk);
    walk.entered.pop();
    result
}
