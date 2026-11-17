//! Purpose: what "Overwrite", "Skip" and "Only newer" do when the name is
//! already taken — merge a folder into a folder, replace a file with a file,
//! and refuse everything that would remove what the transfer does not bring.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::copy::{CopyPrompt, ExistingFiles, Purpose};
use crate::utils::LinkPolicy;

/// One end of a transfer, as far as taking the other's place goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    /// A real folder: another folder merges into it.
    Folder,
    /// A file, a link or anything else: only another of these replaces it.
    Single,
}

impl Shape {
    /// The source as the copy will treat it: a link the dialog does not
    /// follow is copied as a link, so it is not a folder here either.
    pub(super) fn of_source(source: &Path, settings: &CopyPrompt) -> Self {
        let is_link = fs::symlink_metadata(source).is_ok_and(|meta| meta.is_symlink());
        let followed = !is_link || crate::utils::follows_link(settings.rules().links, source);
        if followed && source.is_dir() {
            Self::Folder
        } else {
            Self::Single
        }
    }

    /// What is already at `dest`, never following a link: a link to a folder
    /// is replaced as a link, not merged into.
    pub(super) fn of_existing(dest: &Path) -> io::Result<Self> {
        Ok(if fs::symlink_metadata(dest)?.is_dir() {
            Self::Folder
        } else {
            Self::Single
        })
    }
}

/// Refuse a file and a folder taking each other's place: either way, the
/// one already there would be removed for something of another kind.
pub(super) fn reject_mixed_shapes(source: Shape, existing: Shape) -> io::Result<()> {
    let refusal = match (source, existing) {
        (Shape::Single, Shape::Folder) => "a file or link cannot overwrite a folder",
        (Shape::Folder, Shape::Single) => "a folder cannot overwrite a file or link",
        _ => return Ok(()),
    };
    Err(io::Error::new(io::ErrorKind::InvalidInput, refusal))
}

/// Refuse to merge `source` into a folder that holds it: `pkg/pkg` onto `pkg`
/// would write into the tree it is reading, and a move would take its own
/// source apart.
pub(super) fn reject_merge_into_holder(source: &Path, dest: &Path) -> io::Result<()> {
    let (Ok(real_source), Ok(real_dest)) = (source.canonicalize(), dest.canonicalize()) else {
        return Ok(()); // unreadable paths fail with a real error further down
    };
    if real_source.starts_with(&real_dest) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy or move a folder onto the folder that holds it",
        ));
    }
    Ok(())
}

/// Copy or move what the folder `source` holds into the folder `dest`, each
/// entry by the dialog's "already existing files" choice, as Far, mc and
/// Explorer do. What only `dest` holds is left alone, and so are `dest`'s own
/// times and rights: it is not the folder the transfer brought.
///
/// One entry that fails does not stop the rest; the first error is returned.
/// Otherwise the answer is how many entries, at any depth, the choice left
/// alone: the status line has to say so, or "Moved to …" reads as if all of
/// it went. A moved folder is removed once it is empty, and kept when
/// something in it was skipped.
pub(super) fn merge_into(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<usize> {
    let mut guard = MergeGuard::new(source, dest, settings.rules().links);
    merge_tree(source, dest, settings, &mut guard)
}

#[derive(Default)]
struct MergeGuard {
    entered: Vec<PathBuf>,
    output: Option<PathBuf>,
}

impl MergeGuard {
    fn new(source: &Path, dest: &Path, links: LinkPolicy) -> Self {
        if links == LinkPolicy::Recreate {
            return Self::default();
        }
        Self {
            entered: source.canonicalize().ok().into_iter().collect(),
            output: dest.canonicalize().ok(),
        }
    }

    fn check(&self, source: &Path) -> io::Result<PathBuf> {
        let real = source.canonicalize()?;
        if self
            .output
            .as_ref()
            .is_some_and(|out| real.starts_with(out) || out.starts_with(&real))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} leads into the copy itself", source.display()),
            ));
        }
        if self.entered.iter().any(|open| open.starts_with(&real)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("symbolic link loop at {}", source.display()),
            ));
        }
        Ok(real)
    }
}

fn merge_tree(
    source: &Path,
    dest: &Path,
    settings: &CopyPrompt,
    guard: &mut MergeGuard,
) -> io::Result<usize> {
    let mut first_error = None;
    let mut skipped = 0_usize;
    for entry in fs::read_dir(source)? {
        let result =
            entry.and_then(|e| merge_entry(&e.path(), &dest.join(e.file_name()), settings, guard));
        match result {
            Ok(left) => skipped = skipped.saturating_add(left),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    if settings.purpose == Purpose::Move {
        remove_if_emptied(source)?;
    }
    Ok(skipped)
}

/// Place one entry of a merge; how many entries the choice left alone.
fn merge_entry(
    source: &Path,
    dest: &Path,
    settings: &CopyPrompt,
    guard: &mut MergeGuard,
) -> io::Result<usize> {
    let checked = (settings.rules().links == LinkPolicy::Follow
        && Shape::of_source(source, settings) == Shape::Folder)
        .then(|| guard.check(source))
        .transpose()?;
    if crate::utils::name_is_taken(dest) {
        if settings.existing != ExistingFiles::Ask
            && (
                Shape::of_source(source, settings),
                Shape::of_existing(dest)?,
            ) == (Shape::Folder, Shape::Folder)
        {
            super::placement::reject_transfer_onto_itself(source, dest)?;
            reject_merge_into_holder(source, dest)?;
            if let Some(real) = checked {
                guard.entered.push(real);
                let res = merge_tree(source, dest, settings, guard);
                guard.entered.pop();
                return res;
            }
            return merge_tree(source, dest, settings, guard);
        }
        super::place_over_existing(source, dest, settings).map(|placed| placed.skipped())
    } else {
        super::place(source, dest, settings).map(|()| 0)
    }
}

fn remove_if_emptied(folder: &Path) -> io::Result<()> {
    match fs::remove_dir(folder) {
        Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        other => other,
    }
}
