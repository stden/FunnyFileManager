//! Purpose: The reports a copy, move, delete or search makes while it runs.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::super::{Operation, Phase, Snapshot};
use super::measure::measure;
#[cfg(test)]
pub(in crate::progress) use super::measure::FORCE_MEASURE_ENTRY_ERROR;
use super::{update, update_at, with_screen, Run, Shown};

/// Start showing `operation` on `sources`. A copy or a delete measures them
/// first, reporting as it goes, so the dialog can say how far along it is.
pub fn begin(operation: Operation, sources: &[PathBuf]) -> Run {
    let started = with_screen(|screen| {
        if screen.shown.is_some() {
            return false;
        }
        screen.shown = Some(Shown::new(operation, sources, Instant::now()));
        true
    })
    .unwrap_or(false);
    if started && operation.scans_first() {
        for source in sources {
            measure(source, operation);
        }
        update(|shown| {
            shown.phase = Phase::Working;
            shown.prescanned = true;
        });
    }
    Run { owns: started }
}

/// The operation is at `path` now.
pub fn entering(path: &Path) {
    update(|shown| {
        shown.set_current(path);
        shown.transfer = None;
    });
}

/// A copy wrote `bytes` more.
pub fn copied(bytes: u64) {
    update(|shown| {
        if shown.operation != Operation::Delete {
            shown.done.bytes = shown.done.bytes.saturating_add(bytes);
        }
        shown.copy_bytes(bytes);
    });
}

/// A copy finished one file or link.
pub fn file_copied() {
    if super::transfer::is_staging() {
        return;
    }
    update(|shown| {
        shown.finish_file();
        if shown.operation == Operation::Copy {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// `path` was removed: one file less for a delete, and a move's source
/// going for a move across filesystems.
pub fn removed(path: &Path) {
    update_at(Some(path), |shown| {
        if shown.operation == Operation::Delete {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// A move has to copy `source` across filesystems: its bytes join what the
/// dialog counts, so a big folder moved to another disk fills the bar as
/// it is copied.
pub fn copying_across(source: &Path) {
    if !is_shown() {
        return;
    }
    measure(source, Operation::Move);
}

/// One of the selected entries is done. A move counts them, since a rename
/// moves a whole folder in one step; a copy and a delete count every file.
pub fn source_done() {
    update(|shown| {
        shown.finish_file();
        if matches!(shown.operation, Operation::Move | Operation::GitPull) {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// A search looked at one more entry.
pub fn examined() {
    update(|shown| shown.total.files = shown.total.files.saturating_add(1));
}

/// A search found one more match.
pub fn matched() {
    update(|shown| shown.done.files = shown.done.files.saturating_add(1));
}

/// Nothing changed, but the work goes on: the dialog's clock still moves
/// while one big file is read for a search's text.
pub fn tick() {
    update(|_| ());
}

/// Draw `snapshot`, which another process sent: the administrator helper
/// measures and throttles its own operation, and this one only shows it.
pub fn relay(snapshot: &Snapshot) {
    with_screen(|screen| {
        screen.painter.paint(snapshot);
        screen.painted = true;
    });
}

/// Whether an operation is being shown on this thread.
#[must_use]
pub fn is_shown() -> bool {
    with_screen(|screen| screen.shown.is_some()).unwrap_or(false)
}
