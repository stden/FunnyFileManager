//! Purpose: Deferred dirty-repository probe state for a panel's current directory.

use std::{
    collections::{BTreeSet, HashSet},
    mem,
    path::{Path, PathBuf},
};

use super::{DirEntry, FileName, Panel, PanelMetadata};

/// Uncached child Git repositories waiting for background `git status` checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum DirtyProbe {
    #[default]
    Complete,
    Pending(Vec<FileName>),
}

impl DirtyProbe {
    #[must_use]
    pub(crate) fn from_pending(names: Vec<FileName>) -> Self {
        if names.is_empty() {
            Self::Complete
        } else {
            Self::Pending(names)
        }
    }
}

impl PanelMetadata {
    /// Trim pending probes to visible directories and keep prior dirty badges
    /// provisionally during an in-place reload so rows do not flicker clean.
    pub(super) fn prepare_dirty_probe(
        &mut self,
        entries: &[DirEntry],
        previous_dirty: &BTreeSet<FileName>,
    ) {
        let DirtyProbe::Pending(mut names) = mem::take(&mut self.dirty_probe) else {
            return;
        };
        let visible: HashSet<&str> = entries
            .iter()
            .filter(|entry| entry.kind.is_dir() && entry.name != "..")
            .map(|entry| entry.name.as_str())
            .collect();
        names.retain(|name| visible.contains(name.as_str()));
        for name in &names {
            if previous_dirty.contains(name) {
                self.dirty_repos.insert(name.clone());
            }
        }
        self.dirty_probe = DirtyProbe::from_pending(names);
    }
}

impl Panel {
    /// Claim any unprobed child repositories queued on the last directory load.
    pub(crate) fn take_dirty_probe(&mut self) -> Option<(PathBuf, Vec<FileName>)> {
        match mem::take(&mut self.metadata.dirty_probe) {
            DirtyProbe::Complete => None,
            DirtyProbe::Pending(names) => Some((self.path.clone(), names)),
        }
    }

    /// Record one background `git status` answer when the panel still shows `parent`.
    /// Returns `true` only when the visible dirty-folder set actually changed.
    pub(crate) fn apply_dirty_probe(
        &mut self,
        parent: &Path,
        name: &FileName,
        dirty: bool,
    ) -> bool {
        if self.path != parent {
            return false;
        }
        let listed = self
            .entries
            .iter()
            .any(|entry| entry.kind.is_dir() && entry.name == name.as_str());
        if !listed {
            return false;
        }
        if dirty {
            self.metadata.dirty_repos.insert(name.clone())
        } else {
            self.metadata.dirty_repos.remove(name)
        }
    }
}
