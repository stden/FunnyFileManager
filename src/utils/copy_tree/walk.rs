//! Purpose: the recursive, loop-guarded walk that copies a directory tree entry by entry.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::super::keep_times::{keep_times, Times};
use super::steps::{copy_file_checked, create_dir_all_checked, entry_file_type, read_dir_entry};
use super::{copy_link, follows_link, CopyRules, LinkPolicy};
use crate::utils::FileMode;

#[cfg(any(test, tarpaulin))]
use crate::utils::{forced_copy_step, COPY_DIR_ALL_FAIL_RECURSE};

/// What the walk must not enter twice, or at all.
///
/// Empty under [`LinkPolicy::Recreate`], where no link is followed and a real
/// directory tree cannot contain itself.
pub(super) struct Walk {
    /// The canonical path of every directory on the way down.
    entered: Vec<PathBuf>,
    /// The copy's own output. A link that reaches into it would have the walk
    /// copying what it is writing — every level a fresh path, so the `entered`
    /// stack alone never notices.
    output: Option<PathBuf>,
}

impl Walk {
    pub(super) const fn unguarded() -> Self {
        Self {
            entered: Vec::new(),
            output: None,
        }
    }

    pub(super) fn guarded(src: &Path, dst: &Path) -> Self {
        Self {
            entered: src.canonicalize().ok().into_iter().collect(),
            output: canonical_target(dst),
        }
    }

    /// Why this directory must not be walked, if it must not be.
    fn refusal(&self, real: &Path, src: &Path) -> Option<io::Error> {
        if self
            .output
            .as_ref()
            .is_some_and(|out| real.starts_with(out) || out.starts_with(real))
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

fn canonical_target(dst: &Path) -> Option<PathBuf> {
    dst.canonicalize().ok().or_else(|| {
        dst.ancestors().find_map(|ancestor| {
            let real = ancestor.canonicalize().ok()?;
            Some(real.join(dst.strip_prefix(ancestor).ok()?))
        })
    })
}

pub(super) fn copy_tree(
    src: &Path,
    dst: &Path,
    rules: CopyRules,
    walk: &mut Walk,
) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        if forced_copy_step() == COPY_DIR_ALL_FAIL_RECURSE
            && src
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == ".ffm_force_copy_dir_error")
        {
            return Err(io::Error::other("forced recursive copy failure"));
        }
    }

    crate::progress::entering(src);
    // Read before the walk: reading the folder moves its access time.
    let source = fs::metadata(src)?;
    create_dir_all_checked(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = read_dir_entry(entry)?;
        let file_type = entry_file_type(&entry)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_symlink() && !follows_link(rules.links, &src_path) {
            copy_link(&src_path, &dst_path, rules)?;
        } else if file_type.is_dir() || (file_type.is_symlink() && src_path.is_dir()) {
            // `is_dir` follows the link, which is the right question to ask
            // once the policy says to follow it.
            descend(&src_path, &dst_path, rules, walk)?;
        } else {
            copy_file_checked(&src_path, &dst_path, rules)?;
        }
    }
    finish_folder(dst, &source, rules)
}

/// Give a copied folder, now that its contents are in, what the rules keep
/// of its source: the times first, then the mode, which may take away the
/// write permission the times would otherwise need.
fn finish_folder(dst: &Path, source: &fs::Metadata, rules: CopyRules) -> io::Result<()> {
    if rules.times == Times::Source {
        keep_times(dst, source)?;
    }
    if rules.folder_modes == FileMode::Source {
        restate_mode(dst, source)?;
    }
    Ok(())
}

#[cfg(unix)]
fn restate_mode(dst: &Path, source: &fs::Metadata) -> io::Result<()> {
    fs::set_permissions(dst, source.permissions())
}

/// Windows has no mode bits to carry, and a folder's read-only attribute is
/// not a permission there; the ACL comes from where the copy was made.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
const fn restate_mode(_dst: &Path, _source: &fs::Metadata) -> io::Result<()> {
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
fn descend(src: &Path, dst: &Path, rules: CopyRules, walk: &mut Walk) -> io::Result<()> {
    if rules.links == LinkPolicy::Recreate {
        // Nothing to guard against, and nothing to pay for it: a link never
        // reaches this function under `Recreate`, so every descent is into a
        // real subdirectory and a real directory tree cannot contain itself.
        return copy_tree(src, dst, rules, walk);
    }
    let real = src.canonicalize()?;
    if let Some(refusal) = walk.refusal(&real, src) {
        return Err(refusal);
    }
    walk.entered.push(real);
    let result = copy_tree(src, dst, rules, walk);
    walk.entered.pop();
    result
}
