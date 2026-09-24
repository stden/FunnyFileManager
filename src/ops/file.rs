//! Purpose: Canonical file operations: create, copy, move, mkdir, and delete.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(any(test, tarpaulin))]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::copy::Purpose;
use crate::models::{App, Overlay};
use crate::status::{FailedEntry, Landed, Status};
use crate::utils::{
    copy_dir_all_with, copy_file, copy_symlink, refresh_panels, unique_destination, CopyRules,
};
use destination::Destination;

mod destination;

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
/// user their data is intact when it is gone. F8 on a directory the *other*
/// panel is sitting inside no longer fails that reload — the panel moves up to
/// the nearest folder that exists — but a folder that cannot be read still
/// does. That stale listing is for Ctrl+R to report, on purpose, because there
/// the refresh *is* the operation.
fn refresh_after_operation(app: &mut App) {
    drop(refresh_panels(app));
}

pub fn get_source_paths(app: &App) -> Vec<PathBuf> {
    app.get_active_panel_ref().action_paths()
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
fn copy_entry(source: &Path, dest: &Path, rules: CopyRules) -> io::Result<()> {
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
        && !crate::utils::follows_link(rules.links, source)
    {
        return copy_symlink(source, dest);
    }

    if source.is_dir() {
        return copy_directory_entry(source, dest, rules);
    }

    // Refuses a FIFO, socket or device before opening it or claiming `dest`.
    copy_file(source, dest, rules.modes)
}

fn copy_directory_entry(source: &Path, dest: &Path, rules: CopyRules) -> io::Result<()> {
    // Exclusive claim of the directory name, then fill it. `copy_dir_all_with`
    // is happy if `dest` already exists (it `create_dir_all`s).
    fs::create_dir(dest)?;
    let result = copy_dir_all_with(source, dest, rules);
    if result.is_err() {
        // Best effort: the copy's own error is the one worth reporting.
        drop(fs::remove_dir_all(dest));
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
        match copy_entry(source, &dest, CopyRules::PLAIN) {
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
        match copy_entry(source, &dest, CopyRules::PLAIN) {
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
#[derive(Default)]
struct BatchOutcome {
    count: usize,
    first_dest: Option<PathBuf>,
    errors: Vec<FailedEntry>,
}

impl BatchOutcome {
    fn record(&mut self, source: &Path, result: io::Result<PathBuf>) {
        match result {
            Ok(dest) => {
                self.first_dest.get_or_insert(dest);
                self.count += 1;
            }
            // Which file, and why — kept apart until something decides how to
            // show them. Formatting here threw the two away in one string.
            Err(error) => self.errors.push(FailedEntry::new(source, &error)),
        }
    }
}

fn transfer_all(
    sources: &[PathBuf],
    target_dir: &Path,
    transfer: fn(&Path, &Path) -> io::Result<PathBuf>,
) -> BatchOutcome {
    let mut outcome = BatchOutcome::default();
    for source in sources {
        outcome.record(source, transfer(source, target_dir));
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
        app.set_status(Status::Transferred {
            purpose: Purpose::Copy,
            landed: Landed::of(outcome.count, outcome.first_dest),
            skipped: 0,
            renamed: 0,
        });
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
        app.set_status(Status::Transferred {
            purpose: Purpose::Move,
            landed: Landed::of(outcome.count, outcome.first_dest),
            skipped: 0,
            renamed: 0,
        });
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
    let files_to_delete = get_source_paths(app);
    if files_to_delete.is_empty() {
        return Ok(());
    }

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

mod merge;
mod replace;

#[cfg(test)]
mod tests_destinations;
#[cfg(test)]
mod tests_exhaustion;
#[cfg(test)]
mod tests_mkdir_errors;
#[cfg(test)]
#[cfg(unix)]
mod tests_modes;
#[cfg(test)]
#[cfg(unix)]
mod tests_non_utf8_names;
#[cfg(test)]
mod tests_overwrite_failures;
#[cfg(test)]
mod tests_overwrite_safety;
#[cfg(test)]
mod tests_refresh_failures;
#[cfg(test)]
#[cfg(unix)]
mod tests_sparse;
#[cfg(test)]
#[cfg(unix)]
mod tests_special_files;
#[cfg(test)]
mod tests_symlinks;
#[cfg(test)]
mod tests_transfer_policies;

/// Carry out what the F5/F6 dialog was filled in with.
///
/// The destination is whatever was typed, which may be neither panel — that is
/// the reason the dialog exists — and may be a folder or a new name; see
/// [`Destination`]. A relative path is taken against the active panel,
/// because that is where the user is looking when they type one.
pub fn copy_with(app: &mut App, settings: &crate::copy::CopyPrompt) {
    let sources = get_source_paths(app);
    if sources.is_empty() {
        return;
    }

    let base = &app.get_active_panel_ref().path;
    let destination = Destination::parse(&settings.destination, base, sources.len());
    let mut outcome = DialogOutcome::default();
    for source in &sources {
        outcome.record(source, copy_one(source, &destination, settings));
    }
    report_copy_outcome(app, &outcome, settings.purpose);
    refresh_after_operation(app);
}

/// Where one entry of a dialog transfer ended up.
enum Placed {
    /// Under the name it was sent to.
    AsSent(PathBuf),
    /// Beside something already there, under a free name.
    Beside(PathBuf),
    /// Left alone, as the "already existing files" choice said.
    Skipped,
}

/// What a dialog transfer achieved, item by item.
#[derive(Default)]
struct DialogOutcome {
    batch: BatchOutcome,
    /// Left alone by the policy. Not an error and not a transfer either, so
    /// it is counted apart, or the status line would claim work not done.
    skipped: usize,
    /// Put beside an existing file under a new name — which the status line
    /// has to say, because the name the user looks for is not there.
    renamed: usize,
}

impl DialogOutcome {
    fn record(&mut self, source: &Path, result: io::Result<Placed>) {
        let result = match result {
            Ok(Placed::Skipped) => {
                self.skipped += 1;
                return;
            }
            Ok(Placed::Beside(dest)) => {
                self.renamed += 1;
                Ok(dest)
            }
            Ok(Placed::AsSent(dest)) => Ok(dest),
            Err(error) => Err(error),
        };
        self.batch.record(source, result);
    }
}

fn report_copy_outcome(app: &mut App, outcome: &DialogOutcome, purpose: Purpose) {
    let batch = &outcome.batch;
    app.get_active_panel().clear_marks();
    if let Some(first_error) = batch.errors.first().cloned() {
        let (count, error_count) = (batch.count, batch.errors.len());
        app.set_status(match purpose {
            Purpose::Copy => Status::CopiedWithErrors {
                count,
                error_count,
                first_error,
            },
            Purpose::Move => Status::MovedWithErrors {
                count,
                error_count,
                first_error,
            },
        });
        return;
    }
    app.set_status(Status::Transferred {
        purpose,
        landed: Landed::of(batch.count, batch.first_dest.clone()),
        skipped: outcome.skipped,
        renamed: outcome.renamed,
    });
}

/// Transfer one entry to where the dialog sends it.
fn copy_one(
    source: &Path,
    destination: &Destination,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    let (folder, name) = destination.for_source(source);
    reject_transfer_into_itself(source, folder)?;
    fs::create_dir_all(folder)?;

    let dest = folder.join(name);
    if dest.exists() {
        return place_over_existing(source, &dest, settings);
    }
    place(source, &dest, settings)?;
    Ok(Placed::AsSent(dest))
}

/// `dest` is taken: do what the "already existing files" choice says.
///
/// Skipping is a normal outcome, not a failure — reporting it as an error
/// would put a red count on a deliberate choice. Overwriting never removes
/// what the transfer does not bring; see [`merge`].
fn place_over_existing(
    source: &Path,
    dest: &Path,
    settings: &crate::copy::CopyPrompt,
) -> io::Result<Placed> {
    use crate::copy::ExistingFiles;
    use merge::Shape;

    let shapes = (
        Shape::of_source(source, settings),
        Shape::of_existing(dest)?,
    );
    let folders = shapes == (Shape::Folder, Shape::Folder);
    match settings.existing {
        ExistingFiles::Skip => return Ok(Placed::Skipped),
        // Two folders merge, and "only newer" is decided file by file inside.
        ExistingFiles::OnlyNewer if !folders && !is_newer(source, dest) => {
            return Ok(Placed::Skipped);
        }
        _ => {}
    }
    reject_transfer_onto_itself(source, dest)?;
    if settings.existing == ExistingFiles::Ask {
        // Not yet a per-file prompt. Until it is, the non-destructive reading
        // of "ask" is taken: nothing already there is touched, and the new one
        // lands beside it under a free name.
        let parent = dest.parent().unwrap_or(dest);
        let beside = unique_destination(parent, dest.file_name().unwrap_or_default())?;
        place(source, &beside, settings)?;
        return Ok(Placed::Beside(beside));
    }
    merge::reject_mixed_shapes(shapes.0, shapes.1)?;
    if folders {
        merge::reject_merge_into_holder(source, dest)?;
        merge::merge_into(source, dest, settings)?;
    } else {
        replace::entry(source, dest, settings)?;
    }
    Ok(Placed::AsSent(dest.to_path_buf()))
}

/// Refuse a transfer whose destination *is* its source.
///
/// Both panels start in the same directory, so the dialog's default
/// destination is often the source's own. Far refuses a copy or a move onto
/// itself whatever the "already existing files" choice: "Overwrite" would
/// remove the file and then copy from what is no longer there, and "Ask"
/// would rename a moved file to `name_1` and call that a move.
fn reject_transfer_onto_itself(source: &Path, dest: &Path) -> io::Result<()> {
    let same = source == dest
        || replace::are_hard_links(source, dest)
        || matches!(
            (source.canonicalize(), dest.canonicalize()),
            (Ok(real_source), Ok(real_dest)) if real_source == real_dest
        );
    if same {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy or move onto itself",
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
    if settings.purpose == Purpose::Move && rename_or_fail(source, dest).is_ok() {
        return Ok(());
    }
    copy_entry(source, dest, settings.rules())?;
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

/// Restate the source's permission bits on `dest` when the dialog says Copy.
///
/// Every file was already given its mode as it was written (see
/// [`crate::copy::AccessRights::file_mode`]). What is left is a copied
/// folder, which is created afresh and can only take a read-only source's
/// mode once its contents are in. Default and Inherit leave it as created:
/// the kernel applied the umask then, and a `chmod` now would skip it.
#[cfg(unix)]
fn apply_rights(source: &Path, dest: &Path, rights: crate::copy::AccessRights) -> io::Result<()> {
    if rights != crate::copy::AccessRights::Copy {
        return Ok(());
    }
    fs::set_permissions(dest, fs::metadata(source)?.permissions())
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the Unix signature, where the operation can fail"
)]
fn apply_rights(
    _source: &Path,
    _dest: &Path,
    _rights: crate::copy::AccessRights,
) -> io::Result<()> {
    // Windows has no mode bits to carry; the ACL comes from the destination
    // directory whatever the dialog says.
    Ok(())
}
