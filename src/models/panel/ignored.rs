//! Purpose: Deferred `git check-ignore` probe state for a panel's current directory.

use std::{
    collections::BTreeSet,
    mem,
    path::{Path, PathBuf},
};

use super::{DirEntry, FileName, Panel, PanelMetadata};

/// One directory entry candidate for a deferred `git check-ignore` probe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IgnoredCandidate {
    pub(crate) name: FileName,
    pub(crate) is_dir: bool,
}

/// Uncached directory entries waiting for background `git check-ignore` checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum IgnoredProbe {
    #[default]
    Complete,
    Pending(Vec<IgnoredCandidate>),
}

impl IgnoredProbe {
    #[must_use]
    pub fn from_pending(candidates: Vec<IgnoredCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Complete
        } else {
            Self::Pending(candidates)
        }
    }
}

impl PanelMetadata {
    /// Keep prior ignored marks provisionally during an in-place reload so rows
    /// do not flicker bright while the background probe refreshes the cache.
    pub(super) fn prepare_ignored_probe(
        &mut self,
        entries: &[DirEntry],
        previous_ignored: &BTreeSet<FileName>,
    ) {
        if matches!(self.ignored_probe, IgnoredProbe::Complete) {
            return;
        }
        for entry in entries.iter().filter(|entry| entry.name != "..") {
            if previous_ignored.contains(entry.name.as_str()) {
                self.ignored.insert(FileName::new(entry.name.clone()));
            }
        }
    }
}

impl Panel {
    /// Whether git ignores a directory entry, or it is the repository's `.git`.
    pub(crate) fn is_ignored(&self, name: &str) -> bool {
        self.metadata.ignored.contains(name)
    }

    /// Claim any unprobed entries queued for `git check-ignore` on the last load.
    pub(crate) fn take_ignored_probe(&mut self) -> Option<(PathBuf, Vec<IgnoredCandidate>)> {
        match mem::take(&mut self.metadata.ignored_probe) {
            IgnoredProbe::Complete => None,
            IgnoredProbe::Pending(candidates) => Some((self.path.clone(), candidates)),
        }
    }

    /// Apply one background `git check-ignore` answer when the panel still shows `directory`.
    /// Returns `true` only when the visible ignored-entry set actually changed.
    pub(crate) fn apply_ignored_probe(
        &mut self,
        directory: &Path,
        ignored: &BTreeSet<FileName>,
    ) -> bool {
        if self.path != directory {
            return false;
        }
        let next: BTreeSet<FileName> = self
            .entries
            .iter()
            .filter(|entry| {
                entry.name != ".."
                    && (entry.name == ".git" || ignored.contains(entry.name.as_str()))
            })
            .map(|entry| FileName::new(entry.name.clone()))
            .collect();
        if self.metadata.ignored == next {
            return false;
        }
        self.metadata.ignored = next;
        true
    }
}
