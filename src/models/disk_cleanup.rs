//! Purpose: State for the Disk Cleanup dialog (`:cleanup`).

use std::path::PathBuf;

use super::FileSize;

mod kind;
mod rows;

pub use kind::{CleanupKind, TempScope};

/// Availability and measurement state of a cleanup candidate on disk.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    #[default]
    Absent,
    Present {
        size: Option<FileSize>,
        checked: bool,
    },
}

/// One item in the cleanup checklist.
#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    pub label: &'static str,
    pub path: PathBuf,
    pub kind: CleanupKind,
    pub state: CandidateState,
}

impl CleanupCandidate {
    #[must_use]
    pub const fn exists(&self) -> bool {
        matches!(self.state, CandidateState::Present { .. })
    }

    #[must_use]
    pub const fn is_checked(&self) -> bool {
        matches!(self.state, CandidateState::Present { checked: true, .. })
    }

    #[must_use]
    pub const fn size(&self) -> Option<FileSize> {
        match self.state {
            CandidateState::Present { size, .. } => size,
            CandidateState::Absent => None,
        }
    }

    pub const fn toggle_checked(&mut self) {
        if let CandidateState::Present { checked, .. } = &mut self.state {
            *checked = !*checked;
        }
    }
}

/// Which control has the cursor inside the cleanup dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupField {
    Item(usize),
    Run,
    Cancel,
}

/// All state owned by the Disk Cleanup overlay.
#[derive(Debug, Clone)]
pub struct DiskCleanupDialog {
    pub candidates: Vec<CleanupCandidate>,
    pub field: CleanupField,
}

impl DiskCleanupDialog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            candidates: rows::build_candidates(),
            field: CleanupField::Item(0),
        }
    }

    #[must_use]
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.candidates
            .iter()
            .filter(|c| c.is_checked())
            .map(|c| c.path.clone())
            .collect()
    }

    #[must_use]
    pub fn selected_bytes(&self) -> u64 {
        self.candidates
            .iter()
            .filter(|c| c.is_checked())
            .filter_map(CleanupCandidate::size)
            .map(FileSize::bytes)
            .sum()
    }
}

impl Default for DiskCleanupDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cache_rows;
