#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
pub(super) use unix::{read, write};
#[cfg(windows)]
pub(super) use windows::{read, write};
