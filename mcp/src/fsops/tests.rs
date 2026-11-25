//! Unit tests for the guarded filesystem operations.
//!
//! Each child module covers one operation; the mode probe they share lives here.

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;

mod copying;
mod moving;
#[cfg(unix)]
mod readonly;
mod removal;
mod replacement;

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o7777
}

#[test]
fn parent_dir_defaults_to_current_directory_for_bare_names() {
    use super::parent_dir;
    use std::path::Path;

    assert_eq!(parent_dir(Path::new("plain.txt")), Path::new("."));
    assert_eq!(parent_dir(Path::new("")), Path::new("."));
    assert_eq!(parent_dir(Path::new("/tmp/plain.txt")), Path::new("/tmp"));
}
