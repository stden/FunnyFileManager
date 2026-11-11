//! Purpose: give a copy its source's access and modification times — through
//! the file being written, or by path for a folder or a link, never
//! following the link.

use std::{fs, io, path::Path};

/// Which times a copied entry ends up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Times {
    /// When the copy was made, as `cp` without `-p` leaves them.
    Fresh,
    /// The source's access and modification times, as `cp -p` and `mv`
    /// keep them — on every file, folder and link the copy makes.
    Source,
}

// Makes keeping a copy's times fail: the one metadata step of a copy that
// no file a test can make will refuse. Thread-local, as the copy runs on
// the calling thread and parallel tests must not see it.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_TIMES_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Fail here when a test forced it; see `FORCE_TIMES_FAILURE`.
#[cfg_attr(
    not(any(test, tarpaulin)),
    expect(
        clippy::unnecessary_wraps,
        reason = "outside tests the seam never fails, but its callers are the same"
    )
)]
fn unless_forced() -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_TIMES_FAILURE.get() {
        return Err(io::Error::other("forced times failure"));
    }
    Ok(())
}

/// The access and modification times `source` records, for a handle to set.
pub(super) fn file_times(source: &fs::Metadata) -> io::Result<fs::FileTimes> {
    unless_forced()?;
    Ok(fs::FileTimes::new()
        .set_accessed(source.accessed()?)
        .set_modified(source.modified()?))
}

/// Give the folder or link at `path` the times `source` records.
///
/// By path (`utimensat` with `AT_SYMLINK_NOFOLLOW`), which asks only for
/// ownership: a folder the copy is about to make read-only, or a link, has
/// no handle to write through. A link's own times are set, never its
/// target's — setting them through the link would restamp whatever file it
/// happens to point at.
#[cfg(unix)]
pub(super) fn keep_times(path: &Path, source: &fs::Metadata) -> io::Result<()> {
    use rustix::fs::{utimensat, AtFlags, Timestamps, CWD};

    unless_forced()?;
    let timestamps = Timestamps {
        last_access: timespec(source.accessed()?)?,
        last_modification: timespec(source.modified()?)?,
    };
    utimensat(CWD, path, &timestamps, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)
}

/// Windows sets a folder's times through a `FILE_WRITE_ATTRIBUTES` handle,
/// which only backup semantics can open on a folder. A link keeps the times
/// it was made with: opening it without following it is not worth the
/// privilege creating it already needed.
#[cfg(windows)]
pub(super) fn keep_times(path: &Path, source: &fs::Metadata) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES,
    };

    unless_forced()?;
    if source.is_symlink() {
        return Ok(());
    }
    fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .set_times(file_times(source)?)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn keep_times(_path: &Path, _source: &fs::Metadata) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "times cannot be kept on this platform",
    ))
}

/// A time as `utimensat` reads it.
#[cfg(unix)]
#[expect(
    clippy::map_err_ignore,
    reason = "`TryFromIntError` says only \"out of range\", which the message already does"
)]
pub fn timespec(time: std::time::SystemTime) -> io::Result<rustix::fs::Timespec> {
    use rustix::fs::Timespec;

    let converted = match time.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(after) => Timespec::try_from(after),
        Err(before) => Timespec::try_from(before.duration()).map(|span| -span),
    };
    converted.map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "time out of range"))
}
