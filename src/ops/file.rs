//! File operations: create, copy, move, mkdir, and delete.

use std::fmt::Write as _;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Status};
use crate::utils::{
    copy_dir_all_with, copy_symlink, refresh_panels, unique_destination, LinkPolicy,
};

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
    // Canonical paths, so `..` and symlinked panel paths cannot slip past. The
    // target may not exist yet — a typed `project/archive` is created on the
    // way — so its closest existing ancestor stands in for it; otherwise a
    // nested destination inside the source would slip past this check and be
    // created before the copy itself refused.
    let (Ok(real_source), Some(real_target)) = (
        source.canonicalize(),
        crate::utils::canonical_existing_ancestor(target_dir),
    ) else {
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
fn copy_entry(source: &Path, dest: &Path, links: LinkPolicy) -> io::Result<()> {
    #[cfg(any(test, tarpaulin))]
    if FORCE_TRANSFER_COLLISION.get() {
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
    if fs::symlink_metadata(source).is_ok_and(|metadata| metadata.is_symlink())
        && !crate::utils::follows_link(links, source)
    {
        return copy_symlink(source, dest);
    }

    if source.is_dir() {
        return copy_directory_entry(source, dest, links);
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

fn copy_directory_entry(source: &Path, dest: &Path, links: LinkPolicy) -> io::Result<()> {
    // Exclusive claim of the directory name, then fill it. `copy_dir_all_with`
    // is happy if `dest` already exists (it `create_dir_all`s).
    fs::create_dir(dest)?;
    let result = copy_dir_all_with(source, dest, links);
    if result.is_err() {
        let _ = fs::remove_dir_all(dest);
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
        match copy_entry(source, &dest, LinkPolicy::Recreate) {
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
        match copy_entry(source, &dest, LinkPolicy::Recreate) {
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
    if !app
        .settings
        .enabled(crate::menu::SettingsPage::Confirmations)
    {
        return handle_delete(app);
    }
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
mod tests_mkdir_errors;
#[cfg(test)]
mod tests_refresh_failures;
#[cfg(test)]
mod tests_symlinks;
#[cfg(test)]
mod tests_transfer_policies;

/// Carry out what the F5/F6 dialog was filled in with.
///
/// The destination is whatever was typed, which may be neither panel — that is
/// the reason the dialog exists. A relative path is taken against the active
/// panel, because that is where the user is looking when they type one.
pub fn copy_with(app: &mut App, settings: &crate::copy::CopyPrompt) {
    let sources = get_source_paths(app);
    if sources.is_empty() {
        return;
    }

    let typed = Path::new(settings.destination.trim());
    let target_dir = if typed.is_absolute() {
        typed.to_path_buf()
    } else {
        app.get_active_panel_ref().path.join(typed)
    };

    let mut outcome = BatchOutcome {
        count: 0,
        first_dest: None,
        errors: Vec::new(),
    };
    let mut skipped = 0usize;
    for source in &sources {
        match copy_one(source, &target_dir, settings) {
            Ok(Some(dest)) => {
                if outcome.first_dest.is_none() {
                    outcome.first_dest = Some(dest);
                }
                outcome.count += 1;
            }
            // The policy said to leave this one alone. Not an error, and not a
            // copy either, so it is counted separately or the status line
            // would claim work that did not happen.
            Ok(None) => skipped += 1,
            Err(error) => outcome.errors.push(FailedEntry::new(source, &error)),
        }
    }

    report_copy_outcome(app, &outcome, skipped, settings.existing);
    refresh_after_operation(app);
}

fn report_copy_outcome(
    app: &mut App,
    outcome: &BatchOutcome,
    skipped: usize,
    existing: crate::copy::ExistingFiles,
) {
    use crate::copy::ExistingFiles;
    app.get_active_panel().clear_marks();
    if let Some(first_error) = outcome.errors.first() {
        app.set_status(Status::CopiedWithErrors {
            count: outcome.count,
            error_count: outcome.errors.len(),
            first_error: first_error.clone(),
        });
    } else {
        let mut text =
            format_operation_status(outcome.count, "Copied", outcome.first_dest.as_deref());
        if skipped > 0 {
            let _ = write!(text, ", {skipped} skipped");
        }
        if existing == ExistingFiles::Ask {
            text.push_str(" (existing files kept, copies renamed)");
        }
        app.set_status(Status::CommandOutput(text));
    }
}

/// Copy one entry according to the dialog's collision policy.
///
/// `Ok(None)` means the policy said to leave it: skipping is a normal outcome,
/// not a failure, and reporting it as an error would put a red count on a
/// deliberate choice.
fn copy_one(
    source: &Path,
    target_dir: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Option<PathBuf>> {
    use crate::copy::ExistingFiles;

    reject_transfer_into_itself(source, target_dir)?;
    fs::create_dir_all(target_dir)?;

    let file_name = source.file_name().unwrap_or_default();
    let dest = target_dir.join(file_name);

    if dest.exists() {
        match settings.existing {
            // Not yet a per-file prompt. Until it is, the non-destructive
            // reading of "ask" is taken: nothing already there is touched, and
            // the copy lands beside it under a free name.
            ExistingFiles::Ask => {
                let dest = unique_destination(target_dir, file_name)?;
                place(source, &dest, settings)?;
                return Ok(Some(dest));
            }
            ExistingFiles::Skip => return Ok(None),
            ExistingFiles::OnlyNewer if !is_newer(source, &dest) => return Ok(None),
            ExistingFiles::Overwrite | ExistingFiles::OnlyNewer => {
                reject_transfer_onto_itself(source, &dest)?;
                remove_existing(&dest)?;
            }
        }
    }

    place(source, &dest, settings)?;
    Ok(Some(dest))
}

/// Refuse to overwrite `dest` when it *is* `source`.
///
/// Both panels start in the same directory, so the F5 dialog's default
/// destination is often the source's own directory. With "Overwrite" the file
/// there would be removed first and then copied from what is no longer there,
/// leaving an empty file — or, for a move, nothing at all. Far Manager refuses
/// this outright.
fn reject_transfer_onto_itself(source: &Path, dest: &Path) -> io::Result<()> {
    let same = source == dest
        || matches!(
            (source.canonicalize(), dest.canonicalize()),
            (Ok(real_source), Ok(real_dest)) if real_source == real_dest
        );
    if same {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy a file onto itself",
        ));
    }
    Ok(())
}

/// Put `source` at `dest`, copying or moving as the dialog said.
///
/// A move is tried as a rename first — within one filesystem that is the whole
/// operation and it is atomic. Across filesystems `rename` fails, and the only
/// thing left is to copy and then remove, which is what every mover does.
fn place(source: &Path, dest: &Path, settings: &crate::copy::CopyPrompt) -> io::Result<()> {
    use crate::copy::Purpose;

    if settings.purpose == Purpose::Move && rename_or_fail(source, dest).is_ok() {
        return Ok(());
    }
    copy_entry(source, dest, settings.link_policy())?;
    finish(source, dest, settings)?;
    if settings.purpose == Purpose::Move {
        remove_existing(source)?;
    }
    Ok(())
}

/// Whether `source` was modified after `dest`, for the "only newer" policy.
///
/// Unreadable times mean "copy it": refusing to act because a timestamp could
/// not be read would silently skip the file the user asked about.
fn is_newer(source: &Path, dest: &Path) -> bool {
    let time = |path: &Path| fs::metadata(path).and_then(|meta| meta.modified()).ok();
    match (time(source), time(dest)) {
        (Some(from), Some(to)) => from > to,
        _ => true,
    }
}

/// Remove whatever is at `dest` so a copy can take its place.
fn remove_existing(dest: &Path) -> io::Result<()> {
    if fs::symlink_metadata(dest)?.is_dir() {
        fs::remove_dir_all(dest)
    } else {
        fs::remove_file(dest)
    }
}

/// Apply the dialog's post-copy options to what was just written.
fn finish(source: &Path, dest: &Path, settings: &crate::copy::CopyPrompt) -> io::Result<()> {
    if settings.timestamps {
        let modified = fs::metadata(source)?.modified()?;
        super::timestamps::open(dest)?.set_modified(modified)?;
    }
    apply_rights(source, dest, settings.rights)
}

/// Give `dest` the permissions the dialog asked for.
///
/// `Default` leaves whatever `fs::copy` produced. `Copy` restates the source's
/// mode explicitly — which is what `fs::copy` already does for a plain file,
/// but not for a directory it had to create. `Inherit` drops back to what a
/// newly created file in that directory would get, which is the umask.
#[cfg(unix)]
fn apply_rights(source: &Path, dest: &Path, rights: crate::copy::AccessRights) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    use crate::copy::AccessRights;

    /// What a new file gets before the umask is applied.
    const FRESH: u32 = 0o666;

    match rights {
        AccessRights::Default => Ok(()),
        AccessRights::Copy => {
            let mode = fs::metadata(source)?.permissions().mode();
            fs::set_permissions(dest, fs::Permissions::from_mode(mode))
        }
        AccessRights::Inherit => {
            let executable = fs::metadata(dest)?.is_dir();
            let mode = if executable { 0o777 } else { FRESH };
            fs::set_permissions(dest, fs::Permissions::from_mode(mode))
        }
    }
}

#[cfg(not(unix))]
fn apply_rights(
    _source: &Path,
    _dest: &Path,
    _rights: crate::copy::AccessRights,
) -> io::Result<()> {
    // Windows has no mode bits to carry; the ACL comes from the destination
    // directory whatever the dialog says.
    Ok(())
}
