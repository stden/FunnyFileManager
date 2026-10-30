//! Purpose: Safe wrappers for the process and OS queries several modules share.
//!
//! Who am I, may I write this, kill that process group, signal or renice that
//! process, how big is a page, how fast does the clock tick, which drives
//! exist, hide that file, what does this registry value hold: each is asked
//! here once, so callers get a plain Rust answer and never write `unsafe`. On
//! Unix the answers come from `rustix`, whose calls are safe; the Win32 calls
//! are raw FFI, reviewed in `windows`.

#[cfg(any(windows, test))]
mod registry_text;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(any(windows, test))]
pub use registry_text::{RegistryString, RegistryStringKind};
#[cfg(target_os = "linux")]
pub use unix::ProcessHandle;
#[cfg(unix)]
pub use unix::{check_write_access, effective_uid, is_root, kill_process_group, real_uid};
#[cfg(all(unix, any(target_os = "linux", test)))]
pub use unix::{clock_ticks_per_second, page_size};
#[cfg(unix)]
pub use unix::{hang_up_process_group, has_exited};
#[cfg(all(unix, any(target_os = "linux", test)))]
pub use unix::{set_niceness, signal_process};
#[cfg(windows)]
pub use windows::{accept, client_process_id, create_pipe, open_pipe, server_process_id};
#[cfg(windows)]
pub use windows::{
    broadcast_environment_change, read_registry_string, update_registry_string, RegistryHive,
};
#[cfg(windows)]
pub use windows::{is_elevated, launch_as_administrator, ElevatedProcess};
#[cfg(windows)]
pub use windows::{logical_drives, set_file_attributes};

/// Whether this process can overwrite anything. Unix-only here: Windows
/// elevation is a different question, asked by `is_elevated`.
#[cfg(not(unix))]
pub const fn is_root() -> bool {
    false
}

#[cfg(test)]
mod tests;
#[cfg(test)]
#[cfg(unix)]
mod tests_groups;
#[cfg(test)]
#[cfg(unix)]
mod tests_kernel_constants;
#[cfg(test)]
#[cfg(unix)]
mod tests_processes;
#[cfg(test)]
#[cfg(windows)]
mod tests_registry;
#[cfg(test)]
#[cfg(windows)]
mod tests_windows;
