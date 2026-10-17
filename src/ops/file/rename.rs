//! Purpose: The rename seam shared by copy and move, including the test switches
//! that make a rename fail as a cross-device move or as a refusal.

use std::{fs, io, path::Path};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(test, tarpaulin))]
pub static FORCE_RENAME_FAILURE: AtomicBool = AtomicBool::new(false);

// Makes every `copy_entry` report `AlreadyExists`, simulating the race where
// another process claims each destination between the uniqueness check and
// the copy — the only way to drive the transfer loops to exhaustion.
// Thread-local, unlike `FORCE_RENAME_FAILURE`: transfers run on the calling
// thread, and a process-global flag would sabotage every copy test running
// in parallel with the exhaustion tests.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_TRANSFER_COLLISION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// Makes every `rename_or_fail` fail the way a user who may not write to the
// source's folder fails (`EACCES`), which root never does. Thread-local for
// the reason `FORCE_TRANSFER_COLLISION` is.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub static FORCE_RENAME_DENIAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Mutex to serialize tests that depend on the global `FORCE_RENAME_FAILURE` flag.
#[cfg(test)]
pub static RENAME_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Try to rename (move) a file or directory; returns an error on failure.
/// In test mode, can be forced to fail as a cross-device rename does, or as
/// a refused one does.
pub fn rename_or_fail(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_RENAME_DENIAL.get() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "forced rename denial",
        ));
    }
    #[cfg(any(test, tarpaulin))]
    let forced = FORCE_RENAME_FAILURE.load(Ordering::SeqCst);

    #[cfg(not(any(test, tarpaulin)))]
    let forced = false;

    if forced {
        // What the kernel answers for a rename between two filesystems.
        Err(io::Error::new(
            io::ErrorKind::CrossesDevices,
            "forced rename failure",
        ))
    } else {
        fs::rename(source, dest)
    }
}

/// What renaming an entry to its destination settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Renamed {
    /// The entry is under its new name; the move is done.
    Moved,
    /// The two names are on different filesystems. Only a copy followed by a
    /// removal can move the entry, which is what `mv` does then too.
    AcrossFilesystems,
}

/// Move `source` to `dest` by renaming it, telling a rename across
/// filesystems apart from a rename that was refused.
///
/// Only the first may turn into a copy: `EXDEV` is the kernel saying "not
/// with rename". Any other failure — no permission on the source's folder, a
/// busy mount point, a read-only filesystem — is the move's own failure, and
/// copying anyway used to leave a full copy at the destination beside a
/// source that could not be removed, under a status that said "Moved 0".
pub(super) fn rename_within(source: &Path, dest: &Path) -> io::Result<Renamed> {
    match rename_or_fail(source, dest) {
        Ok(()) => Ok(Renamed::Moved),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            Ok(Renamed::AcrossFilesystems)
        }
        Err(error) => Err(error),
    }
}
