//! Purpose: Narrow a test path's permission bits for as long as a guard lives,
//! and put them back when it drops — after a failed assertion as well.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

/// A path whose mode a test narrowed, restored when the guard drops.
///
/// A directory left at 0o000 (or 0o555 with entries in it) cannot be emptied
/// by its owner, so the `TempDir` above it fails to remove itself and stays
/// behind in `$TMPDIR` on every non-root run. Restoring by hand at the end of
/// a test misses every run that panics first; a guard does not. Declare it
/// after the `TempDir`, so it drops first.
#[derive(Debug)]
pub struct ModeGuard {
    path: PathBuf,
    original: fs::Permissions,
}

impl ModeGuard {
    /// Set `path` to `mode` until the guard drops.
    pub fn set(path: &Path, mode: u32) -> io::Result<Self> {
        let original = fs::metadata(path)?.permissions();
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        Ok(Self {
            path: path.to_owned(),
            original,
        })
    }
}

impl AsRef<Path> for ModeGuard {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for ModeGuard {
    fn drop(&mut self) {
        // Nothing can be done with the error: a panic in `drop` would abort
        // the run, and the path can only have been removed by the test itself.
        drop(fs::set_permissions(&self.path, self.original.clone()));
    }
}
