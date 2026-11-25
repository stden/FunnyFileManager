//! Purpose: Configure an owned private PTY for interruptible, nonblocking capture.

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::{io, os::fd::RawFd};

pub fn nonblocking_terminal_fd(master: &dyn portable_pty::MasterPty) -> io::Result<RawFd> {
    nonblocking_fd(master.as_raw_fd())
}

fn nonblocking_fd(fd: Option<RawFd>) -> io::Result<RawFd> {
    let fd = fd.ok_or_else(|| io::Error::other("Private terminal has no fd"))?;
    let flags = OFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFL).map_err(io::Error::from)?);
    fcntl(fd, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK)).map_err(io::Error::from)?;
    Ok(fd)
}

#[cfg(test)]
mod tests;
