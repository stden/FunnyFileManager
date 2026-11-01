//! Purpose: Poll one PTY reader with bounded waits while retaining its master fd.

use super::super::master::Master;

pub(super) struct Lease {
    #[cfg(unix)]
    master: Master,
}

impl Lease {
    #[cfg(unix)]
    pub fn new(master: &Master) -> Self {
        Self {
            master: master.clone(),
        }
    }

    #[cfg(not(unix))]
    pub const fn new(_: &Master) -> Self {
        Self {}
    }
}

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(super) use unix::Reader;
#[cfg(unix)]
pub(super) use unix::Reader as Capture;
#[cfg(not(unix))]
pub(super) use Lease as Capture;
