//! Purpose: a file with no name until it is whole — what a staged overwrite
//! is written into, so that a process killed half-way leaves nothing behind.

use std::{fs, io, path::Path};

/// A new, empty file in `folder` that has no name yet, or `None` where one
/// cannot be made: anything but Linux, a filesystem without `O_TMPFILE`
/// (NFS, FAT), or a `/proc` too absent to name it by once it is whole. The
/// caller then stages under a name, which reports any real refusal itself.
///
/// The kernel frees such a file when its last handle closes, so a copy cut
/// short by a crash, `kill` or a closed terminal leaves no trace. A named
/// temporary file stayed in the destination folder for good (`.ffm-copy-*`).
/// Created with 0666 before the umask, as a new file would be.
#[cfg(target_os = "linux")]
#[must_use]
pub fn unnamed_file_in(folder: &Path) -> Option<fs::File> {
    use rustix::fs::{openat, Mode, OFlags, CWD};

    if !Path::new("/proc/self/fd").is_dir() {
        return None;
    }
    let flags = OFlags::WRONLY | OFlags::TMPFILE | OFlags::CLOEXEC;
    openat(CWD, folder, flags, Mode::from_raw_mode(0o666))
        .ok()
        .map(fs::File::from)
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn unnamed_file_in(_folder: &Path) -> Option<fs::File> {
    None
}

/// Give the unnamed `file` the name `path`, in the folder it was made in.
/// A name already taken fails with `AlreadyExists`, as an exclusive create
/// does, so the caller's "pick another name" loop works unchanged.
#[cfg(target_os = "linux")]
pub fn name_unnamed(file: &fs::File, path: &Path) -> io::Result<()> {
    use rustix::fs::{linkat, AtFlags, CWD};
    use std::os::fd::AsRawFd;

    let handle = format!("/proc/self/fd/{}", file.as_raw_fd());
    linkat(CWD, handle.as_str(), CWD, path, AtFlags::SYMLINK_FOLLOW).map_err(io::Error::from)
}

#[cfg(not(target_os = "linux"))]
pub fn name_unnamed(_file: &fs::File, _path: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "files without a name exist only on Linux",
    ))
}
