//! Purpose: Copy one entry under a name claimed exclusively — a link as a
//! link, a folder with its contents, a file by its bytes — and leave nothing
//! under that name when the copy fails.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// What a copy is for, which decides what it keeps of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// A copy beside the source: files keep their mode, as `fs::copy` leaves
    /// them; folders are made like any new folder; nothing keeps its times.
    Copy,
    /// The copy that stands in for a rename across filesystems: every file
    /// and folder keeps its mode and its times, as `mv` keeps them. A link
    /// keeps the times it was made with — `std` sets no time on a link
    /// without following it, and a link's own times are not worth a
    /// dependency.
    Move,
}

/// Copy `src` to `dst`, which must not exist yet, preserving what the entry
/// *is*.
///
/// Every name is claimed with an exclusive create (`create_new`,
/// `create_dir`, `symlink`), which fails with [`io::ErrorKind::AlreadyExists`]
/// on anything already there — a dangling link included. `fs::copy` would
/// follow a link at the destination and write the file it names, which for
/// a link planted inside the root may be anywhere on the machine; a copy of
/// a file onto itself would truncate it before reading a byte; and a folder
/// copied onto a folder would merge into it, replacing files inside.
///
/// WHY `symlink_metadata` and never `is_dir()`/`exists()`: those follow the
/// link. Copying through one would pull in the bytes it points at — content
/// from outside the server root, landing inside it — and a link to a
/// directory would drag the whole target tree along. Reproducing the link
/// as a link is also what keeps a dangling link copyable at all.
pub fn copy_entry(src: &Path, dst: &Path, purpose: Purpose) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.is_symlink() {
        return copy_symlink(src, dst);
    }
    if meta.is_dir() {
        return copy_dir(src, dst, &meta, purpose);
    }
    if !meta.is_file() {
        // A FIFO would block until a writer appeared, a device would copy
        // until the disk filled: only a regular file is opened.
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("not a regular file, cannot copy: {}", src.display()),
        ));
    }
    copy_file(src, dst, &meta, purpose)
}

/// Recreate `src` at `dst` as a link to the same target, without reading it.
#[cfg(unix)]
fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(src)?, dst)
}

/// Same, for Windows — where the link itself carries the file/directory
/// distinction, so the kind has to be chosen from what the link reaches.
#[cfg(windows)]
fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    if src.is_dir() {
        std::os::windows::fs::symlink_dir(&target, dst)
    } else {
        std::os::windows::fs::symlink_file(&target, dst)
    }
}

/// Claim the folder name and fill it. A failure inside removes the folder:
/// the exclusive create made it and everything in it, and a partial copy
/// under the final name is indistinguishable from a complete one.
fn copy_dir(src: &Path, dst: &Path, meta: &fs::Metadata, purpose: Purpose) -> io::Result<()> {
    reject_copy_into_itself(src, dst)?;
    fs::create_dir(dst)?;
    let filled = fill_dir(src, dst, meta, purpose);
    if filled.is_err() {
        // Best effort: the copy's own error is the one worth reporting.
        drop(fs::remove_dir_all(dst));
    }
    filled
}

/// Copy the contents, then give the folder what the purpose keeps — its
/// mode last, once its contents are in: a read-only folder could not be
/// filled, and writing each entry into it moves its modification time.
fn fill_dir(src: &Path, dst: &Path, meta: &fs::Metadata, purpose: Purpose) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        copy_entry(&entry.path(), &dst.join(entry.file_name()), purpose)?;
    }
    if purpose == Purpose::Move {
        keep_times(&open_folder(dst)?, meta)?;
        fs::set_permissions(dst, meta.permissions())?;
    }
    Ok(())
}

/// Refuse to copy a folder into itself or into one of its children: the
/// walk would descend into the copy it is writing and duplicate the tree
/// until the path length gave out. The destination does not exist yet, so
/// its closest existing ancestor stands in for it.
fn reject_copy_into_itself(src: &Path, dst: &Path) -> io::Result<()> {
    let (Ok(real_src), Some(real_dst)) = (src.canonicalize(), canonical_existing_ancestor(dst))
    else {
        return Ok(()); // an unreadable path fails with its own error further down
    };
    if real_dst.starts_with(&real_src) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot copy a directory into itself: {}", src.display()),
        ));
    }
    Ok(())
}

fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find_map(|ancestor| ancestor.canonicalize().ok())
}

/// Claim the file name and copy the bytes. A copy that fails half-way is
/// removed: ours to remove, since the exclusive create made it.
fn copy_file(src: &Path, dst: &Path, meta: &fs::Metadata, purpose: Purpose) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)?;
    let written = write_copy(&mut input, &mut output, meta, purpose);
    if written.is_err() {
        drop(fs::remove_file(dst));
    }
    written
}

/// The bytes, then the times a move keeps — through the handle, before the
/// mode, so that a read-only mode cannot stand in their way.
fn write_copy(
    input: &mut fs::File,
    output: &mut fs::File,
    meta: &fs::Metadata,
    purpose: Purpose,
) -> io::Result<()> {
    io::copy(input, output)?;
    if purpose == Purpose::Move {
        keep_times(output, meta)?;
    }
    output.set_permissions(meta.permissions())
}

/// Give the copy the access and modification times its source records.
fn keep_times(copy: &fs::File, source: &fs::Metadata) -> io::Result<()> {
    let times = fs::FileTimes::new()
        .set_accessed(source.accessed()?)
        .set_modified(source.modified()?);
    copy.set_times(times)
}

/// A handle to set a folder's times through.
#[cfg(unix)]
fn open_folder(path: &Path) -> io::Result<fs::File> {
    fs::File::open(path)
}

/// Windows opens a folder only with backup semantics, and sets its times
/// through a handle with the write-attributes right.
#[cfg(windows)]
fn open_folder(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}
