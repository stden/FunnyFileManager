//! Purpose: Single-pass streaming progress for directory and batch deletions,
//! avoiding an upfront filesystem walk and copying path names only when due.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::super::{Operation, Phase};
use super::{with_screen, Run, Shown};

/// Begin showing `Operation::Delete` directly in `Phase::Working` without
/// pre-scanning `sources` on disk.
pub fn begin_delete(sources: &[PathBuf]) -> Run {
    let started = with_screen(|screen| {
        if screen.shown.is_some() {
            return false;
        }
        let mut shown = Shown::new(Operation::Delete, sources, Instant::now());
        shown.phase = Phase::Working;
        screen.shown = Some(shown);
        true
    })
    .unwrap_or(false);
    Run { owns: started }
}

/// Record `discovered` and `removed` entries for an active delete, painting
/// and updating `current` only when the throttle interval is due. Returns
/// whether this operation has painted at least one progress frame.
pub fn delete_progress(discovered: u64, removed: u64, current: Option<&Path>) -> bool {
    with_screen(|screen| {
        let Some(shown) = screen.shown.as_mut() else {
            return false;
        };
        if shown.operation == Operation::Delete {
            if !shown.prescanned {
                shown.total.files = shown.total.files.saturating_add(discovered);
            }
            shown.done.files = shown.done.files.saturating_add(removed);
        }
        let now = Instant::now();
        if shown.throttle.due(now, screen.interval) {
            if let Some(path) = current {
                shown.set_current(path);
            }
            shown.throttle.last = Some(now);
            let snapshot = shown.snapshot(now);
            screen.painter.paint(&snapshot);
            screen.painted = true;
        }
        shown.throttle.last.is_some()
    })
    .unwrap_or(false)
}
