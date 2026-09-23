//! Purpose: Provides shared pure utilities for sizes, unique paths, copies, and refreshes.
//!
use crate::models::FileSize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::App;

mod copy_file;
mod copy_tree;

pub(crate) use copy_file::copy_file;
pub use copy_file::FileMode;
pub(crate) use copy_tree::{
    canonical_existing_ancestor, copy_dir_all_with, copy_symlink, follows_link,
};
pub use copy_tree::{copy_dir_all, CopyRules, LinkPolicy};

#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_CREATE: u8 = 1;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_ENTRY: u8 = 2;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_FILE_TYPE: u8 = 3;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_RECURSE: u8 = 4;
#[cfg(any(test, tarpaulin))]
const COPY_DIR_ALL_FAIL_COPY_FILE: u8 = 5;

#[cfg(any(test, tarpaulin))]
use std::sync::{Mutex, MutexGuard, PoisonError};

#[cfg(any(test, tarpaulin))]
static FORCE_REFRESH_ERROR: Mutex<bool> = Mutex::new(false);

#[cfg(any(test, tarpaulin))]
static FORCE_COPY_DIR_ALL_STEP: Mutex<u8> = Mutex::new(0);

/// Poison-tolerant lock of a FORCE_* test flag. A panicking test must not
/// cascade failures into unrelated tests that only read the flag.
#[cfg(any(test, tarpaulin))]
fn force_lock<T>(mutex: &'static Mutex<T>) -> MutexGuard<'static, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Largest file the viewer (F3) and the built-in editor (F4) will load.
///
/// One constant so the guard and the "max N MB" message it produces can never
/// drift apart.
pub const MAX_FILE_SIZE: FileSize = FileSize::from_bytes(10 * 1024 * 1024);

/// Whether the name is already taken.
///
/// `Path::exists` follows links and so calls a *dangling* symlink a free name —
/// but every exclusive create (`create_new`, `create_dir`, `symlink`) still
/// fails on it with `AlreadyExists`. The caller would then retry with the very
/// same name until it ran out of attempts and reported a spurious failure.
/// `symlink_metadata` asks about the name itself, which is the question.
fn name_is_taken(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Generate a unique file path to avoid overwriting existing files.
/// If "test.txt" already exists, returns "`test_1.txt`", "`test_2.txt`", etc.
pub fn unique_destination(base: &Path, file_name: &std::ffi::OsStr) -> io::Result<PathBuf> {
    const MAX_UNIQUE_ATTEMPTS: u32 = 9999;

    let mut dest = base.join(file_name);
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let extension = Path::new(file_name).extension().and_then(|e| e.to_str());
    let mut counter: u32 = 1;
    while name_is_taken(&dest) && counter <= MAX_UNIQUE_ATTEMPTS {
        let new_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem}_{counter}.{ext}"),
            _ => format!("{stem}_{counter}"),
        };
        dest = base.join(new_name);
        counter += 1;
    }
    if name_is_taken(&dest) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "could not find unique name for {} after {MAX_UNIQUE_ATTEMPTS} attempts",
                file_name.display()
            ),
        ));
    }
    Ok(dest)
}

/// Make a symbolic link at `link` pointing at `target`.
///
/// Platform-split because the two systems disagree about whether a link knows
/// what it points at: Unix does not care, Windows has a separate call for a
/// directory link and will not create one without it.
///
/// The target is not checked for existence. A link to something not there yet
/// is a dangling link, which is a thing people deliberately make, and refusing
/// it here would be this program deciding it knows better.
#[cfg(unix)]
pub fn symlink_dir(target: &str, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
pub fn symlink_dir(target: &str, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

pub fn refresh_panels(app: &mut App) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if *force_lock(&FORCE_REFRESH_ERROR) {
        return Err(io::Error::other("forced refresh_panels failure"));
    }
    // Both panels are reloaded unconditionally, and only then is the first
    // failure reported. `?` on the left reload used to skip the right panel
    // entirely, so deleting a directory the *other* panel sits in left the
    // acting panel still listing the entry it had just removed.
    let left = app.left_panel.reload();
    app.left_panel.ensure_selection();
    let right = app.right_panel.reload();
    app.right_panel.ensure_selection();
    left.and(right)
}

#[cfg(test)]
pub(crate) fn reset_forced_errors() {
    *force_lock(&FORCE_COPY_DIR_ALL_STEP) = 0;
    *force_lock(&FORCE_REFRESH_ERROR) = false;
}

#[cfg(test)]
pub(crate) fn set_copy_dir_all_step(step: u8) {
    *force_lock(&FORCE_COPY_DIR_ALL_STEP) = step;
}

#[cfg(test)]
pub(crate) fn set_refresh_panels_error(value: bool) {
    *force_lock(&FORCE_REFRESH_ERROR) = value;
}

/// Serialise tests that call `refresh_panels` — one of them flips
/// `FORCE_REFRESH_ERROR`, and a concurrent test would spuriously fail.
/// Exposed crate-wide so tests in other modules can take the lock too.
#[cfg(any(test, feature = "test-helpers"))]
pub fn refresh_panels_mutex() -> &'static std::sync::Mutex<()> {
    use std::sync::{Mutex, OnceLock};
    static REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    REFRESH_LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_copy_dir_errors;
#[cfg(test)]
mod tests_copy_guards;
#[cfg(test)]
mod tests_error_paths;
#[cfg(test)]
mod tests_refresh_and_format;
