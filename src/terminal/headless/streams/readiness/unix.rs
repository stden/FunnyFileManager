//! Purpose: Rearm readable PTY readiness and return regularly for cancellation checks.

use super::Lease;
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use std::{io, os::fd::RawFd, time::Duration};

pub(in crate::terminal::headless::streams) struct Reader {
    poll: Poll,
    events: Events,
    fd: RawFd,
    _lease: Lease,
}

impl Reader {
    pub fn new(lease: Lease) -> io::Result<Self> {
        let fd = lease.master.nonblocking_fd()?;
        let poll = Poll::new()?;
        poll.registry()
            .register(&mut SourceFd(&fd), Token(0), Interest::READABLE)?;
        Ok(Self {
            poll,
            events: Events::with_capacity(4),
            fd,
            _lease: lease,
        })
    }

    pub fn wait(&mut self) -> io::Result<bool> {
        // A single reader consumes one chunk per event; rearm unread buffered data.
        self.poll
            .registry()
            .reregister(&mut SourceFd(&self.fd), Token(0), Interest::READABLE)?;
        match self
            .poll
            .poll(&mut self.events, Some(Duration::from_millis(10)))
        {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(false),
            Err(error) => Err(error),
            Ok(()) => Ok(self
                .events
                .iter()
                .any(|event| event.is_readable() || event.is_read_closed() || event.is_error())),
        }
    }
}
