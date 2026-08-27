//! Purpose: Provides shared pure utilities for sizes, unique paths, copies, and refreshes.
//!
use crate::models::FileSize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::models::App;

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

fn create_dir_all_checked(dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_CREATE {
            return Err(io::Error::other("forced create_dir_all failure"));
        }
    }
    fs::create_dir_all(dst)
}

#[allow(clippy::missing_const_for_fn)] // Cannot be const when #[cfg(test)] block is present
fn read_dir_entry(entry: io::Result<fs::DirEntry>) -> io::Result<fs::DirEntry> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_ENTRY {
            return Err(io::Error::other("forced read_dir entry failure"));
        }
    }
    entry
}

fn entry_file_type(entry: &fs::DirEntry) -> io::Result<fs::FileType> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_FILE_TYPE {
            return Err(io::Error::other("forced file_type failure"));
        }
    }
    entry.file_type()
}

fn copy_file_checked(src: &Path, dst: &Path) -> io::Result<u64> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_COPY_FILE {
            return Err(io::Error::other("forced copy failure"));
        }
    }
    fs::copy(src, dst)
}

/// Recreate a symlink instead of following it.
///
/// `DirEntry::file_type` does not follow links but `fs::copy` does, so treating
/// a link as a file turns a link farm into full copies of its targets, fails
/// outright on broken or directory links, and — in the move fallback — destroys
/// the originals afterwards. The link structure must survive the copy.
///
/// Creating the link fails with [`io::ErrorKind::AlreadyExists`] when `dst` is
/// taken, exactly like the exclusive create in `ops::file::copy_entry`, so the
/// caller's "pick another name and retry" loop keeps working.
pub(crate) fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, dst)
    }
    #[cfg(windows)]
    {
        // Windows picks the link flavour at creation time, and creating one may
        // need privileges — best effort, the error is reported like any other.
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(target, dst)
        } else {
            std::os::windows::fs::symlink_file(target, dst)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = target;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symlinks are not supported on this platform",
        ))
    }
}

/// Canonical form of the closest existing ancestor of `path`.
/// A copy destination normally does not exist yet, so it cannot be
/// canonicalised directly.
fn canonical_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find_map(|ancestor| ancestor.canonicalize().ok())
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    // Defence in depth: the walk below would descend into the copy it is
    // writing and duplicate the tree until the path length gives out. Callers
    // guard too — this keeps a future caller from re-introducing it.
    if let (Some(real_src), Some(real_dst)) = (
        canonical_existing_ancestor(src),
        canonical_existing_ancestor(dst),
    ) {
        if real_dst.starts_with(&real_src) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot copy a directory into itself",
            ));
        }
    }
    copy_tree(src, dst)
}

fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    {
        let step = force_lock(&FORCE_COPY_DIR_ALL_STEP);
        if *step == COPY_DIR_ALL_FAIL_RECURSE
            && src
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == ".ffm_force_copy_dir_error")
        {
            return Err(io::Error::other("forced recursive copy failure"));
        }
    }

    create_dir_all_checked(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = read_dir_entry(entry)?;
        let file_type = entry_file_type(&entry)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_symlink() {
            copy_symlink(&src_path, &dst_path)?;
        } else if file_type.is_dir() {
            copy_tree(&src_path, &dst_path)?;
        } else {
            copy_file_checked(&src_path, &dst_path)?;
        }
    }
    Ok(())
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
    let left = app.left_panel.load_entries();
    app.left_panel.ensure_selection();
    let right = app.right_panel.load_entries();
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
