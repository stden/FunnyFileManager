//! Purpose: What a batch of copies or moves achieved before the first failure.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::super::remove_moved::LeftBehind;
use crate::status::FailedEntry;
use crate::utils::FailedAt;

/// What a batch of copies/moves achieved. The batch stops at the first item
/// that fails, as Far's does, so the user sees what went wrong before
/// anything else is touched; the failure is collected instead of propagated,
/// so what was done before it is still reported.
#[derive(Default)]
pub(in crate::ops::file) struct BatchOutcome {
    pub(in crate::ops::file) count: usize,
    pub(in crate::ops::file) first_dest: Option<PathBuf>,
    pub(in crate::ops::file) errors: Vec<FailedEntry>,
    /// Selected items not started because an earlier one failed.
    pub(in crate::ops::file) left: usize,
    /// Why the first failure happened, when it was a refusal.
    pub(in crate::ops::file) denial: Option<crate::models::Denial>,
}

impl BatchOutcome {
    pub(in crate::ops::file) fn record(&mut self, source: &Path, result: io::Result<PathBuf>) {
        let error = match result {
            Ok(dest) => return self.landed(dest),
            Err(error) => error,
        };
        // Which file, and why — kept apart until something decides how to
        // show them. Formatting here threw the two away in one string.
        if let Some(left) = LeftBehind::of(&error) {
            // Moved whole, and part of the source stayed: both are true, and
            // the user acts on each — keep the copy, deal with what stayed.
            self.landed(left.moved_to.clone());
            self.errors.push(FailedEntry::new(&left.path, left));
            return;
        }
        let failed = FailedAt::of(&error).map_or(source, |at| at.path());
        if self.errors.is_empty() {
            self.denial = super::super::denied::classify(failed, &error);
        }
        self.errors.push(FailedEntry::new(failed, &error));
    }

    /// Whether the batch has to stop: an item failed.
    pub(in crate::ops::file) fn stopped(&self) -> bool {
        !self.errors.is_empty()
    }

    fn landed(&mut self, dest: PathBuf) {
        self.first_dest.get_or_insert(dest);
        self.count += 1;
    }
}
