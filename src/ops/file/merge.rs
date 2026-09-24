//! Purpose: what "Overwrite" and "Only newer" do when the name is already
//! taken — merge a folder into a folder, replace a file with a file, and
//! refuse everything that would remove what the transfer does not bring.

use std::fs;
use std::io;
use std::path::Path;

use crate::copy::{CopyPrompt, Purpose};

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
/// A moved folder is removed once it is empty, and kept when something in it
/// was skipped.
pub(super) fn merge_into(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    let mut first_error = None;
    for entry in fs::read_dir(source)? {
        let result = entry
            .and_then(|entry| merge_entry(&entry.path(), &dest.join(entry.file_name()), settings));
        if let Err(error) = result {
            first_error.get_or_insert(error);
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    if settings.purpose == Purpose::Move {
        return remove_if_emptied(source);
    }
    Ok(())
}

fn merge_entry(source: &Path, dest: &Path, settings: &CopyPrompt) -> io::Result<()> {
    if fs::symlink_metadata(dest).is_ok() {
        super::place_over_existing(source, dest, settings).map(drop)
    } else {
        super::place(source, dest, settings)
    }
}

fn remove_if_emptied(folder: &Path) -> io::Result<()> {
    match fs::remove_dir(folder) {
        Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Ok(()),
        other => other,
    }
}
