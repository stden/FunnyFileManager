//! Purpose: Commands > Space analysis — measure the active folder, judge each
//! entry against git, and open the dialog. It reads the disk and git and removes
//! nothing: Enter in the dialog only moves the panel's cursor, and F8 stays the
//! user's to press.

mod git;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_repo;

use std::io;
use std::path::Path;
use std::time::Duration;

use crate::models::space_analysis::{SpaceAnalysisDialog, SpaceEntry};
use crate::models::{App, Overlay};
use crate::space_verdict::{classify, EntryFacts, Holding, Link};
use crate::stats::{self, Budget, Measured, Shape, Stop};
use crate::status::Status;

/// What one scan may spend: entries it may visit, and the time it may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Limits {
    pub(super) entries: usize,
    pub(super) time: Duration,
}

/// The limits of a real scan: enough for a source tree, and short enough that
/// the dialog opens without a pause a key press would notice.
const LIMITS: Limits = Limits {
    entries: 100_000,
    time: Duration::from_secs(3),
};

/// Commands > Space analysis: measure the active panel's folder and open the dialog.
pub fn open(app: &mut App) -> io::Result<()> {
    let panel = app.get_active_panel_ref();
    if panel.kind.is_netbox() || !panel.path.is_dir() {
        return Err(io::Error::other("Space analysis needs a local folder"));
    }
    let folder = panel.path.clone();
    let dialog = scan(&folder, LIMITS)?;
    if !dialog.note.is_empty() {
        app.set_status(Status::CommandOutput(dialog.note.clone()));
    }
    app.set_overlay(Overlay::SpaceAnalysis(Box::new(dialog)));
    Ok(())
}

/// Measure `folder` and judge each of its entries. The disk is read through the
/// shared walk and git through its two listings; nothing is changed.
pub(super) fn scan(folder: &Path, limits: Limits) -> io::Result<SpaceAnalysisDialog> {
    let mut budget = Budget::new(limits.entries, Some(limits.time));
    let measured = stats::measure_entries(folder, &mut budget)?;
    let repo = git::survey(folder);
    let unread: usize = measured.iter().map(|entry| entry.unread).sum();
    let entries = measured
        .into_iter()
        .map(|entry| judge(entry, &repo))
        .collect();
    let note = note(budget.stop(), unread, &repo);
    Ok(SpaceAnalysisDialog::new(
        folder.to_path_buf(),
        entries,
        note,
    ))
}

/// Judge one measured entry against git's facts about it.
fn judge(entry: Measured, repo: &git::Repo) -> SpaceEntry {
    let facts = EntryFacts {
        git: repo.facts(&entry.name),
        link: if entry.shape == Shape::Link {
            Link::Symlink
        } else {
            Link::Plain
        },
        holding: if entry.git_entries > 0 {
            Holding::GitMetadata
        } else {
            Holding::Plain
        },
        bytes: entry.bytes,
        name: entry.name,
    };
    SpaceEntry {
        verdict: classify(&facts),
        name: facts.name,
        bytes: facts.bytes,
    }
}

/// The line that says which limit stopped a measurement.
fn stop_note(stop: Stop) -> &'static str {
    match stop {
        Stop::Entries => "The entry budget ran out: sizes are lower bounds.",
        Stop::Time => "The time budget ran out: sizes are lower bounds.",
    }
}

/// Everything the dialog must say about the answer's limits, in one line.
fn note(stop: Option<Stop>, unread: usize, repo: &git::Repo) -> String {
    let unread = (unread > 0)
        .then(|| format!("{unread} folder(s) could not be read: sizes are lower bounds."));
    [
        stop.map(stop_note).map(str::to_owned),
        unread,
        repo.note().map(str::to_owned),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}
