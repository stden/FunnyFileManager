//! File operations: create, copy, move, mkdir, and delete.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};
use crate::utils::{copy_dir_all, copy_symlink, refresh_panels, unique_destination};

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

/// Mutex to serialize tests that depend on the global `FORCE_RENAME_FAILURE` flag.
#[cfg(test)]
pub static RENAME_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Try to rename (move) a file or directory; returns an error on failure.
/// In test mode, can be forced to fail to simulate cross-device moves.
pub fn rename_or_fail(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    let forced = FORCE_RENAME_FAILURE.load(Ordering::SeqCst);

    #[cfg(not(any(test, tarpaulin)))]
    let forced = false;

    if forced {
        Err(io::Error::other("forced rename failure"))
    } else {
        fs::rename(source, dest)
    }
}

/// Reload both panels after an operation that already happened, keeping the
/// operation's own status message.
///
/// WHY the refresh error is dropped — do not turn this back into `?`: by the
/// time it runs, the files are already copied/moved/created/deleted and the
/// status bar already says so. Propagating a *refresh* failure makes the caller
/// replace that with "Delete failed: No such file or directory", telling the
/// user their data is intact when it is gone. No race is needed to hit it: F8 on
/// a directory the *other* panel is sitting inside makes the other panel's
/// reload fail every time. A stale listing is fixed by the next navigation or by
/// Ctrl+R, which reports refresh errors on purpose because there the refresh
/// *is* the operation.
fn refresh_after_operation(app: &mut App) {
    let _ = refresh_panels(app);
}

pub fn get_source_paths(app: &App) -> Vec<PathBuf> {
    let panel = app.get_active_panel_ref();
    if !panel.marked.is_empty() {
        panel
            .marked
            .iter()
            .map(|name| panel.path.join(name))
            .collect()
    } else if let Some(path) = panel.selected_path() {
        vec![path]
    } else {
        Vec::new()
    }
}

pub fn format_operation_status(count: usize, op_name: &str, first_dest: Option<&Path>) -> String {
    if count > 1 {
        format!("{op_name} {count} item(s)")
    } else if let Some(dest) = first_dest {
        format!("{} to {}", op_name, dest.display())
    } else {
        String::new()
    }
}

pub fn handle_create_file(app: &mut App, filename: &str) -> io::Result<()> {
    let filename = filename.trim();
    if filename.is_empty() {
        app.set_status(Status::FilenameEmpty);
        return Ok(());
    }

    let file_path = app.get_active_panel().path.join(filename);
    // Exclusive create: no TOCTOU between exists-check and open (another
    // process cannot slip a file in and have us truncate it).
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
    {
        Ok(_) => {
            app.set_status(Status::CreatedFile(filename.to_owned()));
            refresh_after_operation(app);
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            app.set_status(Status::FileExists(filename.to_owned()));
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Refuse to copy or move a directory into itself or into one of its children.
///
/// The destination comes from the *other* panel, which may well sit inside the
/// directory under the cursor; the recursive copy would then descend into the
/// copy it is writing and duplicate the tree until the path length gives out.
/// Far Manager and mc both refuse this outright.
fn reject_transfer_into_itself(source: &Path, target_dir: &Path) -> io::Result<()> {
    // Canonical paths, so `..` and symlinked panel paths cannot slip past.
    let (Ok(real_source), Ok(real_target)) = (source.canonicalize(), target_dir.canonicalize())
    else {
        return Ok(()); // unreadable paths fail with a real error further down
    };
    if real_target.starts_with(&real_source) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy a directory into itself",
        ));
    }
    Ok(())
}

/// How many times to re-pick a destination when exclusive create races another
/// process that claimed the same free name between `unique_destination` and
/// the open/create.
const MAX_TRANSFER_ATTEMPTS: u32 = 10_000;

/// Copy `source` onto `dest`, removing a half-written `dest` if it fails — a
/// partial copy under the final name is indistinguishable from a complete one.
///
/// Directories are claimed with exclusive `create_dir` first; files with
/// `create_new`. Only paths we successfully claimed are cleaned up on failure
/// (an `AlreadyExists` means another process owns the name — leave it alone).
/// Callers retry on [`io::ErrorKind::AlreadyExists`].
fn copy_entry(source: &Path, dest: &Path) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_TRANSFER_COLLISION.with(std::cell::Cell::get) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "forced destination collision",
        ));
    }
    // The link itself decides, before anything that would follow it: both
    // `Path::is_dir` and `fs::copy` resolve links, so `logs -> /var/log` under
    // the cursor would be transferred as a full copy of the target tree — and
    // the move fallback would then delete the user's link. `copy_dir_all`
    // already recreates nested links; the top-level entry must match.
    if fs::symlink_metadata(source).is_ok_and(|metadata| metadata.is_symlink()) {
        return copy_symlink(source, dest);
    }

    if source.is_dir() {
        // Exclusive claim of the directory name, then fill it. `copy_dir_all`
        // is happy if `dest` already exists (it `create_dir_all`s).
        fs::create_dir(dest)?;
        let result = copy_dir_all(source, dest);
        if result.is_err() {
            let _ = fs::remove_dir_all(dest);
        }
        return result;
    }

    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)?;
    let result = (|| {
        let mut input = fs::File::open(source)?;
        io::copy(&mut input, &mut out)?;
        out.flush()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(dest);
    }
    result
}

/// Remove `path` without ever following it.
///
/// `Path::is_dir` resolves links, so a symlink to a directory would be handed
/// to `fs::remove_dir_all` — which either refuses outright (the link is not a
/// directory) or, on a platform that walks it, takes the *target's* contents
/// with it. `symlink_metadata` keeps the decision on the link itself, so a link
/// is always unlinked and never walked. Used by both the move fallback and F8.
fn remove_path(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn copy_into(source: &Path, target_dir: &Path) -> io::Result<PathBuf> {
    reject_transfer_into_itself(source, target_dir)?;
    let file_name = source.file_name().unwrap_or_default();
    for _ in 0..MAX_TRANSFER_ATTEMPTS {
        let dest = unique_destination(target_dir, file_name)?;
        match copy_entry(source, &dest) {
            Ok(()) => return Ok(dest),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not find exclusive destination for {}",
            file_name.to_string_lossy()
        ),
    ))
}

fn move_into(source: &Path, target_dir: &Path) -> io::Result<PathBuf> {
    reject_transfer_into_itself(source, target_dir)?;
    let file_name = source.file_name().unwrap_or_default();
    for _ in 0..MAX_TRANSFER_ATTEMPTS {
        let dest = unique_destination(target_dir, file_name)?;
        if rename_or_fail(source, &dest).is_ok() {
            return Ok(dest);
        }

        // Cross-device rename (or forced test failure): exclusive copy + delete.
        // `?` after a successful exclusive copy so the source is only removed
        // once the whole copy has landed.
        match copy_entry(source, &dest) {
            Ok(()) => {
                remove_path(source)?;
                return Ok(dest);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "could not find exclusive destination for {}",
            file_name.to_string_lossy()
        ),
    ))
}

/// What a batch of copies/moves achieved. One failing item must not abandon the
/// rest of the batch, so failures are collected instead of propagated.
struct BatchOutcome {
    count: usize,
    first_dest: Option<PathBuf>,
    errors: Vec<FailedEntry>,
}

fn transfer_all(
    sources: &[PathBuf],
    target_dir: &Path,
    transfer: fn(&Path, &Path) -> io::Result<PathBuf>,
) -> BatchOutcome {
    let mut outcome = BatchOutcome {
        count: 0,
        first_dest: None,
        errors: Vec::new(),
    };
    for source in sources {
        match transfer(source, target_dir) {
            Ok(dest) => {
                if outcome.first_dest.is_none() {
                    outcome.first_dest = Some(dest);
                }
                outcome.count += 1;
            }
            // Which file, and why — kept apart until something decides how to
            // show them. Formatting here threw the two away in one string.
            Err(error) => outcome.errors.push(FailedEntry::new(source, &error)),
        }
    }
    outcome
}

pub fn handle_copy(app: &mut App) -> io::Result<()> {
    let source_paths = get_source_paths(app);
    if source_paths.is_empty() {
        return Ok(());
    }

    let target_dir = app.get_inactive_panel().path.clone();
    let outcome = transfer_all(&source_paths, &target_dir, copy_into);

    app.get_active_panel().clear_marks();
    if let Some(first_error) = outcome.errors.first() {
        app.set_status(Status::CopiedWithErrors {
            count: outcome.count,
            error_count: outcome.errors.len(),
            first_error: first_error.clone(),
        });
    } else {
        app.set_status(Status::CommandOutput(format_operation_status(
            outcome.count,
            "Copied",
            outcome.first_dest.as_deref(),
        )));
    }
    refresh_after_operation(app);
    Ok(())
}

pub fn handle_move(app: &mut App) -> io::Result<()> {
    let source_paths = get_source_paths(app);
    if source_paths.is_empty() {
        return Ok(());
    }

    let target_dir = app.get_inactive_panel().path.clone();
    let outcome = transfer_all(&source_paths, &target_dir, move_into);

    app.get_active_panel().clear_marks();
    if let Some(first_error) = outcome.errors.first() {
        app.set_status(Status::MovedWithErrors {
            count: outcome.count,
            error_count: outcome.errors.len(),
            first_error: first_error.clone(),
        });
    } else {
        app.set_status(Status::CommandOutput(format_operation_status(
            outcome.count,
            "Moved",
            outcome.first_dest.as_deref(),
        )));
    }
    refresh_after_operation(app);
    Ok(())
}

pub fn handle_mkdir(app: &mut App) -> io::Result<()> {
    let base_dir = app.get_active_panel().path.clone();
    // Exclusive `create_dir` + retry: no TOCTOU with another process claiming
    // `new_folder` between exists-check and create.
    for counter in 0..=9999 {
        let candidate = if counter == 0 {
            base_dir.join("new_folder")
        } else {
            base_dir.join(format!("new_folder_{counter}"))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => {
                app.set_status(Status::CreatedDir(candidate.display().to_string()));
                refresh_after_operation(app);
                return Ok(());
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not find unique folder name after 9999 attempts",
    ))
}

/// Show delete confirmation dialog (F8)
pub fn handle_delete_confirm(app: &mut App) -> io::Result<()> {
    let panel = app.get_active_panel();

    // Collect files to delete (marked files or selected file)
    let files_to_delete: Vec<PathBuf> = if !panel.marked.is_empty() {
        panel
            .marked
            .iter()
            .map(|name| panel.path.join(name))
            .collect()
    } else if let Some(path) = panel.selected_path() {
        vec![path]
    } else {
        return Ok(());
    };

    app.set_overlay(Overlay::DeleteConfirm(files_to_delete));
    Ok(())
}

/// Actually perform the delete operation
pub fn handle_delete(app: &mut App) -> io::Result<()> {
    let Some(files) = app.take_delete_confirm() else {
        return Ok(());
    };

    let mut deleted_count = 0;
    let mut errors = Vec::new();

    for target in &files {
        // Same rule as the move fallback: never resolve a link before removing
        // it, or F8 on `logs -> /var/log` would attack the target.
        match remove_path(target) {
            Ok(()) => deleted_count += 1,
            Err(error) => errors.push(FailedEntry::new(target, &error)),
        }
    }

    // Clear marks after deletion
    app.get_active_panel().clear_marks();

    if errors.is_empty() {
        app.set_status(Status::Deleted {
            count: deleted_count,
        });
    } else {
        app.set_status(Status::DeletedWithErrors {
            count: deleted_count,
            error_count: errors.len(),
            first_error: errors.first().cloned().unwrap_or_default(),
        });
    }

    refresh_after_operation(app);
    Ok(())
}

#[cfg(test)]
mod tests_exhaustion;
#[cfg(test)]
mod tests_refresh_failures;
#[cfg(test)]
mod tests_symlinks;
