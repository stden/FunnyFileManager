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
    #[cfg(unix)]
    match fs::metadata(path) {
        Ok(metadata) => ensure_regular(&metadata)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let file = match open_without_waiting(path, options) {
        Ok(file) => file,
        Err(error) => {
            #[cfg(not(unix))]
            if let Ok(metadata) = fs::metadata(path) {
                ensure_regular(&metadata)?;
            }
            return Err(error);
        }
    };
    ensure_regular(&file.metadata()?)?;
    Ok(file)
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
