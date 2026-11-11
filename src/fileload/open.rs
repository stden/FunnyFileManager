//! Purpose: Opening a file that must be a regular one — a transfer's source,
//! a lock, a log — without ever waiting on a FIFO put in its place.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

/// Open `path` as `options` say, refusing anything that is not a regular
/// file; a missing one is left to `options` to create or report.
///
/// LIVENESS GUARD — `open` on a FIFO waits for the other end, for reading
/// and for writing alike, and every caller runs on a thread the user is
/// waiting for: the event loop froze with no key able to reach it. A device
/// node would read or take writes forever, and a socket cannot be opened at
/// all. So the type is checked *before* the open; the handle is checked again
/// because the name can be replaced between the two, and the open itself
/// cannot wait, so even that race ends in a refusal.
pub fn open_regular(path: &Path, options: &OpenOptions) -> io::Result<File> {
    open_regular_with_metadata(path, options).map(|(file, _)| file)
}

/// Like [`open_regular`], returning the already-queried handle metadata so
/// callers such as file copy do not issue a second `fstat` on the same handle.
pub fn open_regular_with_metadata(
    path: &Path,
    options: &OpenOptions,
) -> io::Result<(File, fs::Metadata)> {
    #[cfg(unix)]
    reject_existing_non_regular(path)?;

    #[cfg(unix)]
    let file = open_without_waiting(path, options)?;
    #[cfg(not(unix))]
    let file = open_without_waiting(path, options)
        .map_err(|error| prefer_non_regular_error(path, error))?;

    let metadata = file.metadata()?;
    ensure_regular(&metadata)?;
    Ok((file, metadata))
}

#[cfg(unix)]
fn reject_existing_non_regular(path: &Path) -> io::Result<()> {
    match fs::metadata(path) {
        Ok(metadata) => ensure_regular(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(not(unix))]
fn prefer_non_regular_error(path: &Path, open_error: io::Error) -> io::Error {
    if let Ok(metadata) = fs::metadata(path) {
        if let Err(non_regular) = ensure_regular(&metadata) {
            return non_regular;
        }
    }
    open_error
}

fn ensure_regular(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.is_file() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "not a regular file",
    ))
}

/// `O_NONBLOCK` makes no difference to a regular file or its lock; it only
/// stops `open` from waiting for the other end of a FIFO.
#[cfg(unix)]
fn open_without_waiting(path: &Path, options: &OpenOptions) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    options.clone().custom_flags(libc::O_NONBLOCK).open(path)
}

/// Windows keeps its pipes out of the file namespace, so there is nothing
/// here for an `open` to wait on.
#[cfg(not(unix))]
fn open_without_waiting(path: &Path, options: &OpenOptions) -> io::Result<File> {
    options.open(path)
}
