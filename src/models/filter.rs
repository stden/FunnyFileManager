//! Purpose: The file panel filter (`Ctrl+I`) — what a panel leaves out of its
//! listing, and the dialog that edits it.
//!
//! The filter lives on the panel and is applied where the listing is
//! committed, so every route into a directory — a reload, `Enter`, a
//! bookmark — sees the same rows. A filter applied only by the dialog that set
//! it would silently lapse on the next `Ctrl+R`.

use super::DirEntry;
use crate::mask::Mask;

/// What the panel keeps. `Default` is no filtering at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelFilter {
    /// Only names matching this are shown; `None` admits everything.
    pub include: Option<Mask>,
    /// Names matching this are dropped, whatever `include` says.
    pub exclude: Option<Mask>,
    /// Drop names beginning with a dot.
    pub hide_dotfiles: bool,
}

impl PanelFilter {
    /// Whether the filter changes anything — what puts the `*` in the panel
    /// title.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.include.is_some() || self.exclude.is_some() || self.hide_dotfiles
    }

    /// Drop every entry the filter refuses. The parent row is never dropped:
    /// a filter that could hide `..` would trap the user in the directory.
    pub fn apply(&self, entries: &mut Vec<DirEntry>) {
        if !self.is_active() {
            return;
        }
        entries.retain(|entry| self.admits(&entry.name));
    }

    fn admits(&self, name: &str) -> bool {
        if name == ".." {
            return true;
        }
        if self.hide_dotfiles && name.starts_with('.') {
            return false;
        }
        let included = self.include.as_ref().is_none_or(|mask| mask.matches(name));
        let excluded = self.exclude.as_ref().is_some_and(|mask| mask.matches(name));
        included && !excluded
    }
}

/// The `Ctrl+I` dialog: the masks as typed, before they are parsed into a
/// [`PanelFilter`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterPrompt {
    pub include: String,
    pub exclude: String,
    pub hide_dotfiles: bool,
}

#[cfg(test)]
mod tests;
