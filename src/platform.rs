//! Purpose: Safe wrappers for the process and OS queries several modules share.
//!
//! Who am I, may I write this, kill that process group, signal or renice that
//! process, how big is a page, how fast does the clock tick, which drives
//! exist, hide that file: each is asked here once, so callers get a plain Rust
//! answer and never write `unsafe`. On Unix the answers come from `rustix`,
//! whose calls are safe; the Win32 calls are raw FFI, reviewed in `windows`.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use unix::ProcessHandle;
#[cfg(unix)]
pub use unix::{check_write_access, effective_uid, is_root, kill_process_group, real_uid};
#[cfg(unix)]
pub use unix::{clock_ticks_per_second, page_size};
#[cfg(unix)]
pub use unix::{set_niceness, signal_process};
#[cfg(windows)]
pub use windows::{logical_drives, set_file_attributes};

/// Whether this process can overwrite anything. Unix-only here: Windows
/// elevation is a different question, and nothing in the file manager asks it.
#[cfg(not(unix))]
pub const fn is_root() -> bool {
    false
}

#[cfg(test)]
mod tests;
#[cfg(test)]
#[cfg(unix)]
mod tests_processes;
