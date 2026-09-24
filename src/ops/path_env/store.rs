#[cfg(any(windows, test))]
mod registry;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
pub(super) use unix::{read, write};
#[cfg(windows)]
pub(super) use windows::{read, write};

/// What a save says when the stored PATH is no longer the one the editor read.
const CHANGED_OUTSIDE: &str = "PATH changed outside FFM; reopen before saving";
