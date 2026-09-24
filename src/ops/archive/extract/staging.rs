//! Purpose: Move what unpacked in the private staging folder into the
//! destination as a whole — never replacing an entry, never through a link —
//! or not at all.

use std::fs;
use std::io;
use std::path::Path;

/// Move what unpacked in `staging` into `destination`, or nothing at all.
pub(super) fn settle(staging: &Path, destination: &Path) -> io::Result<()> {
    plain_tree(staging)?;
    check_clashes(staging, destination, Path::new(""))?;
    move_in(staging, destination)
}

/// Refuse a staged tree holding anything but plain files and folders. The
/// listing already refused links; this is what stands if one got past it,
/// because every later entry could be written through it.
pub(in crate::ops::archive) fn plain_tree(staging: &Path) -> io::Result<()> {
    plain_below(staging, Path::new(""))
}

#[expect(
    clippy::filetype_is_file,
    reason = "exactly regular files and folders may be extracted; anything else is refused"
)]
fn plain_below(staging: &Path, relative: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staging.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            plain_below(staging, &relative)?;
        } else if !kind.is_file() {
            return Err(io::Error::other(format!(
                "{} is not a plain file or folder; nothing was extracted",
                relative.display()
            )));
        }
    }
    Ok(())
}

/// Refuse the first staged entry the destination already has, unless both
/// are real folders: those are merged, so what is inside them is compared
/// in turn. A link in the destination is never a real folder.
fn check_clashes(staging: &Path, destination: &Path, relative: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staging.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let Some(existing) = existing(&destination.join(&relative))? else {
            continue;
        };
        if !(existing.is_dir() && entry.file_type()?.is_dir()) {
            return Err(clash(&relative, &existing));
        }
        check_clashes(staging, destination, &relative)?;
    }
    Ok(())
}

fn clash(relative: &Path, existing: &fs::Metadata) -> io::Error {
    let why = if existing.is_symlink() {
        "is a link, and extraction never writes through one"
    } else {
        "already exists"
    };
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} {why}; nothing was extracted", relative.display()),
    )
}

/// What is at `path`, the link itself when it is one; `None` when nothing.
fn existing(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Rename each staged entry into place, going down only into the folders
/// both sides have; `check_clashes` has made sure nothing is replaced.
fn move_in(staged: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staged)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if existing(&target)?.is_some_and(|metadata| metadata.is_dir()) {
            move_in(&entry.path(), &target)?;
        } else {
            fs::rename(entry.path(), &target)?;
        }
    }
    Ok(())
}
