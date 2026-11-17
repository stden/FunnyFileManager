//! A self-pipe that becomes readable when the terminal is resized, so the
//! terminal wait wakes for a resize that landed while the event loop was busy.
//!
//! `poll(2)` wakes only for a signal that interrupts it. A `SIGWINCH` that
//! comes while a frame is being drawn is still pending when the next wait
//! starts, and the wait would sleep its whole budget on the terminal before
//! crossterm, which holds that signal, is asked. The handler installed here
//! writes one byte per signal into a pipe the wait polls beside the terminal,
//! so the wait returns at once instead.

use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::OnceLock;

use signal_hook::consts::SIGWINCH;

/// The read end of the pipe. The first call installs the handler, so a
/// `SIGWINCH` raised before that is not caught.
///
/// # Errors
///
/// The pipe could not be made, or the kernel refused the handler.
fn reader() -> io::Result<&'static UnixStream> {
    static READER: OnceLock<io::Result<UnixStream>> = OnceLock::new();
    READER
        .get_or_init(install)
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
}

/// The descriptor a wait polls for a pending resize.
///
/// # Errors
///
/// See [`reader`].
pub(super) fn descriptor() -> io::Result<RawFd> {
    Ok(reader()?.as_raw_fd())
}

/// Empties the pipe. Its bytes say only that a resize came; crossterm reports
/// the resize itself.
///
/// # Errors
///
/// A read that fails for any reason other than an empty pipe.
pub(super) fn drain() -> io::Result<()> {
    let mut stream = reader()?;
    match io::copy(&mut stream, &mut io::sink()) {
        Err(error) if error.kind() != io::ErrorKind::WouldBlock => Err(error),
        _ => Ok(()),
    }
}

fn install() -> io::Result<UnixStream> {
    let (read_end, write_end) = UnixStream::pair()?;
    read_end.set_nonblocking(true)?;
    signal_hook::low_level::pipe::register(SIGWINCH, write_end)?;
    Ok(read_end)
}
