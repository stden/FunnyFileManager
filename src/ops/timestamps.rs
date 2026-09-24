//! Handles for timestamp changes, including Windows directories and read-only files.

use std::{fs, io, path::Path};

use crate::models::RequestedTimes;

#[cfg(windows)]
pub(super) fn open(path: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES,
    };

    fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
pub(super) fn open(path: &Path) -> io::Result<fs::File> {
    fs::File::open(path).or_else(|_| fs::OpenOptions::new().write(true).open(path))
}

/// Set the requested times on `path`, leaving a `None` one as it is.
///
/// Unix writes them by path (`utimensat`), which asks only for ownership.
/// A handle would need a read or write permission the entry may not grant
/// (the owner's own mode-0000 file), and opening a pipe waits for a peer.
#[cfg(unix)]
pub(super) fn set_times(path: &Path, times: RequestedTimes) -> io::Result<()> {
    use rustix::fs::{utimensat, AtFlags, Timestamps, CWD};

    let timestamps = Timestamps {
        last_access: timespec(times.accessed)?,
        last_modification: timespec(times.modified)?,
    };
    utimensat(CWD, path, &timestamps, AtFlags::empty()).map_err(io::Error::from)
}

/// A time as `utimensat` reads it; `None` is `UTIME_OMIT`, "leave it".
#[cfg(unix)]
#[expect(
    clippy::map_err_ignore,
    reason = "`TryFromIntError` says only \"out of range\", which the message already does"
)]
fn timespec(time: Option<std::time::SystemTime>) -> io::Result<rustix::fs::Timespec> {
    use rustix::fs::{Timespec, UTIME_OMIT};

    let Some(time) = time else {
        return Ok(Timespec {
            tv_sec: 0,
            tv_nsec: UTIME_OMIT,
        });
    };
    let converted = match time.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(after) => Timespec::try_from(after),
        Err(before) => Timespec::try_from(before.duration()).map(|span| -span),
    };
    converted.map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "time out of range"))
}

/// Windows writes through a `FILE_WRITE_ATTRIBUTES` handle, which neither
/// the read-only flag nor the entry's other permissions stand in the way of.
#[cfg(not(unix))]
pub(super) fn set_times(path: &Path, times: RequestedTimes) -> io::Result<()> {
    let mut file_times = fs::FileTimes::new();
    if let Some(modified) = times.modified {
        file_times = file_times.set_modified(modified);
    }
    if let Some(accessed) = times.accessed {
        file_times = file_times.set_accessed(accessed);
    }
    open(path)?.set_times(file_times)
}
