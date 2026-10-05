//! Purpose: What a long copy, move, delete or search has done so far.
//!
//! F5, F6, F8 and Alt+F7 report here as they go, and the painter the session
//! installed draws the progress dialog no more often than every
//! [`SHOWN_EVERY`], so the file name in it changes at most ten times a second.
//!
//! Ambient, like the fault injection in `ops::file`: the reports come from
//! deep inside the folder walk and the file copy, and threading a sink through
//! every signature between `copy_with` and `copy_file` would change a dozen of
//! them for one side effect. Everything is thread-local — the operations run
//! on the UI thread — and a report is one thread-local read while nothing is
//! being shown: no painter (every test, unless it installs one) or no
//! operation running.
//!
//! The painter draws the dialog alone, over whatever the last frame left on
//! the screen, because the operation holds `&mut App` while it runs. Once an
//! operation that painted is over, [`take_painted`] tells the event loop to
//! repaint the whole screen.

use std::path::PathBuf;
use std::time::Duration;

/// How long the dialog shows one state before the next may replace it, and
/// how long an operation runs before the dialog first appears: one that is
/// over sooner is never shown at all.
pub const SHOWN_EVERY: Duration = Duration::from_millis(100);

/// Which operation the dialog reports on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Operation {
    Copy,
    Move,
    Delete,
    /// Alt+F7 and find folder walking the tree. It has no total to measure:
    /// `total.files` counts the entries looked at and `done.files` the
    /// matches, so a long search shows it is still going and what it found.
    Find,
}

impl Operation {
    /// The operation an F5/F6 dialog's purpose names.
    #[must_use]
    pub const fn of(purpose: crate::copy::Purpose) -> Self {
        match purpose {
            crate::copy::Purpose::Copy => Self::Copy,
            crate::copy::Purpose::Move => Self::Move,
        }
    }

    /// Whether the sources are measured before the work starts. A move is
    /// mostly renames, which take no time whatever their size, so only what
    /// has to be copied across filesystems is measured, when it comes to it.
    const fn scans_first(self) -> bool {
        match self {
            Self::Copy | Self::Delete => true,
            Self::Move | Self::Find => false,
        }
    }
}

/// Where an operation is: measuring what it has to do, or doing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Scanning,
    Working,
}

/// A count of files and of their bytes. Folders are not counted: they take
/// no time to make or remove, and a tree of empty folders is done when its
/// last one is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tally {
    pub files: u64,
    pub bytes: u64,
}

/// One state of an operation, as the dialog shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub operation: Operation,
    pub phase: Phase,
    /// The file or folder being worked on (or measured).
    pub current: PathBuf,
    pub done: Tally,
    /// What there is to do, as far as it is known: while scanning, what has
    /// been found so far.
    pub total: Tally,
    pub elapsed: Duration,
}

impl Snapshot {
    /// How far along the operation is, in thousandths: by bytes once there
    /// are bytes to count, by files otherwise. Never above 1000, although a
    /// tree can grow while it is copied.
    #[must_use]
    pub fn permille(&self) -> u16 {
        let (done, total) = if self.total.bytes > 0 {
            (self.done.bytes, self.total.bytes)
        } else {
            (self.done.files, self.total.files)
        };
        if total == 0 {
            return 0;
        }
        let permille = (u128::from(done) * 1000 / u128::from(total)).min(1000);
        u16::try_from(permille).unwrap_or(1000)
    }
}

/// Draws the progress dialog. The session installs the one that draws on
/// the terminal; tests install one that records what it was asked to draw.
pub trait Painter {
    fn paint(&mut self, snapshot: &Snapshot);
}

mod engine;

pub use engine::{
    begin, copied, copying_across, entering, examined, file_copied, install, install_with,
    is_shown, matched, relay, removed, source_done, take_painted, tick, Installed, Run,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_copy;
