//! Unit tests for the guarded filesystem operations.
//!
//! Each child module covers one operation; the mode probe they share lives here.

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
mod readonly;
mod removal;
mod replacement;

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}
