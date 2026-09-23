//! Purpose: File attributes (`Ctrl+A`): read the selection into the dialog, and write the dialog back.
//!
//! The only place this feature touches the disk. `open` stats every marked
//! entry so the boxes can say `[x]`, `[ ]` or `[?]` truthfully; `apply` walks
//! the same entries — and, when asked, what is inside them — changing only the
//! bits and times the dialog moved. Each entry is re-read before it is
//! written, because "set the execute bit" on ten files with ten different
//! modes means ten different results, not one mode stamped on all of them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::directory::owner::Owners;
use crate::models::{App, Attributes, AttributesDialog, EntrySnapshot, Overlay, RequestedTimes};
use crate::status::Status;

/// Ctrl+A: open the File attributes dialog on the selection.
///
/// On `..` the dialog is for the folder itself, as Far's is: the row stands
/// for where you are, and its attributes are the ones a cursor there can
/// mean.
pub fn open(app: &mut App) {
    let mut targets = super::file::get_source_paths(app);
    if targets.is_empty() {
        targets.push(app.get_active_panel_ref().path.clone());
    }
    let mut owners = Owners::default();
    let entries: Vec<EntrySnapshot> = targets
        .into_iter()
        .map(|path| snapshot(path, &mut owners))
        .collect();
    app.set_overlay(Overlay::Attributes(Box::new(
        AttributesDialog::from_snapshots(&entries),
    )));
}

/// What the dialog needs to know about one entry. An entry that cannot be
/// read still opens — with nothing set — rather than blocking the dialog for
/// the ones that can; `apply` will report it then.
fn snapshot(path: PathBuf, owners: &mut Owners) -> EntrySnapshot {
    let metadata = fs::metadata(&path).or_else(|_| fs::symlink_metadata(&path));
    let Ok(metadata) = metadata else {
        return EntrySnapshot {
            path,
            ..EntrySnapshot::default()
        };
    };
    EntrySnapshot {
        path,
        bits: Attributes::from_metadata(&metadata).bits(),
        is_dir: metadata.is_dir(),
        owner: owners.of(&metadata),
        group: owners.group_of(&metadata),
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
    }
}

/// How the run went: what was set, what refused, and the first refusal.
#[derive(Debug, Default)]
struct Outcome {
    done: usize,
    failed: usize,
    first_error: Option<String>,
}

impl Outcome {
    fn record(&mut self, path: &Path, result: io::Result<()>) {
        match result {
            Ok(()) => self.done += 1,
            Err(error) => {
                self.failed += 1;
                self.first_error
                    .get_or_insert_with(|| format!("{}: {error}", path.display()));
            }
        }
    }

    fn summary(&self) -> String {
        let done = format!("Attributes set on {} object(s)", self.done);
        match &self.first_error {
            Some(error) => format!("{done}, {} failed: {error}", self.failed),
            None => done,
        }
    }
}

/// Set: write the dialog to every target, then re-list both panels.
pub fn apply(app: &mut App, dialog: &AttributesDialog) {
    let times = match dialog.requested_times() {
        Ok(times) => times,
        Err(refused) => {
            app.set_status(Status::CommandOutput(refused.to_string()));
            return;
        }
    };
    let mut outcome = Outcome::default();
    for path in &dialog.targets {
        apply_path(path, dialog, times, &mut outcome);
    }
    app.set_status(Status::CommandOutput(outcome.summary()));
    app.get_active_panel().clear_marks();
    // A refresh failure is not this operation's news: the status bar already
    // says what was set, and Ctrl+R reports listing trouble on its own.
    let _ = crate::utils::refresh_panels(app);
}

/// One target, and what is inside it when the dialog asked for that.
fn apply_path(
    path: &Path,
    dialog: &AttributesDialog,
    times: RequestedTimes,
    outcome: &mut Outcome,
) {
    outcome.record(path, apply_one(path, dialog, times));
    if !dialog.process_subfolders || !path.is_dir() {
        return;
    }
    let children = match fs::read_dir(path) {
        Ok(children) => children,
        Err(error) => return outcome.record(path, Err(error)),
    };
    for child in children.flatten() {
        apply_path(&child.path(), dialog, times, outcome);
    }
}

/// Times first, then bits. The dialog always offers the current stamps, so
/// Set opens the file to write them; a chmod that drops owner-read would make
/// that open fail for anyone who is not root.
fn apply_one(path: &Path, dialog: &AttributesDialog, times: RequestedTimes) -> io::Result<()> {
    stamp(path, times)?;
    if !dialog.changes_bits() {
        return Ok(());
    }
    let bits = Attributes::from_metadata(&fs::metadata(path)?).bits();
    set_bits(path, dialog.new_bits(bits))
}

fn stamp(path: &Path, times: RequestedTimes) -> io::Result<()> {
    if times.modified.is_none() && times.accessed.is_none() {
        return Ok(());
    }
    let mut file_times = fs::FileTimes::new();
    if let Some(modified) = times.modified {
        file_times = file_times.set_modified(modified);
    }
    if let Some(accessed) = times.accessed {
        file_times = file_times.set_accessed(accessed);
    }
    super::timestamps::open(path)?.set_times(file_times)
}

#[cfg(unix)]
fn set_bits(path: &Path, bits: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(bits & 0o7777))
}

#[cfg(windows)]
fn set_bits(path: &Path, bits: u32) -> io::Result<()> {
    crate::platform::set_file_attributes(path, bits)
}

#[cfg(not(any(unix, windows)))]
fn set_bits(_path: &Path, _bits: u32) -> io::Result<()> {
    Err(io::Error::other(
        "attributes cannot be set on this platform",
    ))
}

#[cfg(test)]
mod tests;
