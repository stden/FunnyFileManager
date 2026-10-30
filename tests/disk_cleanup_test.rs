//! Purpose: the Disk Cleanup dialog's state and cursor, exercised through the
//! public API.
//!
//! Everything here goes through `ops::disk_cleanup` and `models`, so the tests
//! describe what `:cleanup` does rather than how the dialog stores it. The
//! candidate list is replaced with fixtures under a `TempDir`: the real one
//! points at machine-specific caches, which would make the results depend on
//! the developer's home directory.

use std::fs;
use std::path::{Path, PathBuf};

use funny_file_manager::models::{
    App, CandidateState, CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog, FileSize,
    Overlay, Panel,
};
use funny_file_manager::ops::disk_cleanup;
use tempfile::TempDir;

#[path = "disk_cleanup_test/dialog_input.rs"]
mod dialog_input;
#[path = "disk_cleanup_test/measure.rs"]
mod measure;
#[path = "disk_cleanup_test/selection.rs"]
mod selection;

fn candidate(label: &'static str, path: impl Into<PathBuf>) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path: path.into(),
        kind: CleanupKind::Path,
        state: CandidateState::Absent,
    }
}

/// A dialog carrying only the given candidates, with the cursor on the first.
fn dialog_with(candidates: Vec<CleanupCandidate>) -> DiskCleanupDialog {
    let mut dialog = DiskCleanupDialog::new();
    dialog.candidates = candidates;
    dialog.field = CleanupField::Item(0);
    dialog
}
