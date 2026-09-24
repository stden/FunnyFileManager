//! Purpose: the *unlocked* half of a `.kdbx` session — the secret-free entry
//! list, the incremental filter over it, cursor movement, and the revealed
//! password.
//!
//! Pure state: no rendering, no key types, no clock of its own. Every mutating
//! method takes the caller's `Instant` so the idle auto-lock in
//! [`super::KdbxSession::tick`] is driven by real user activity and stays
//! testable without a terminal.

mod accessors;
mod filter;
mod secrets;

use std::path::PathBuf;
use std::time::Instant;

use super::edit::EntryForm;
use super::secret::{MasterKey, Revealed};
use crate::keepass::EntryId;
use crate::keepass::EntrySummary;

/// Cursor motions over the filtered entry list.
///
/// `PageUp` / `PageDown` carry their own step because only the renderer knows
/// how many rows the overlay currently shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Prev,
    Next,
    PageUp(usize),
    PageDown(usize),
}

/// Which entry browser is behind a transient vault dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowsePage {
    Entries,
    Tree,
}

/// The visible unlocked-vault surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Browse(BrowsePage),
    Password(BrowsePage),
    DeleteConfirm(BrowsePage),
}

/// An unlocked vault: the master key plus everything derived from it.
///
/// Holding the key is unavoidable — `crate::keepass::read_entry` re-opens the
/// database on every read — so this type is the blast radius for the session's
/// secrets and is deliberately not constructible without a [`MasterKey`].
///
/// **No `Debug`**: [`MasterKey`] has none, which makes deriving one here a
/// compile error. That is the guarantee, not an oversight.
pub struct Vault {
    master: MasterKey,
    /// Kept alongside the password because the key file is half of the
    /// credentials and every re-read needs both.
    keyfile: Option<PathBuf>,
    /// Secret-free summaries — `crate::keepass::list_entries` never reads a
    /// password, so this list is safe to keep, filter, and render.
    entries: Vec<EntrySummary>,
    filter: String,
    /// Indices into [`Self::entries`] that survive [`Self::filter`].
    matches: Vec<usize>,
    /// Index into [`Self::matches`] (**not** into `entries`).
    selected: usize,
    revealed: Option<Revealed>,
    /// The `F4` edit form, when one is open. It lives here rather than beside
    /// the session so "editing while locked" stays unrepresentable: a `Vault`
    /// exists only inside `KdbxState::Unlocked`. Boxed because the form is a
    /// few hundred bytes of draft text, which inline would tip the unlocked
    /// variant of `KdbxState` over clippy's large-variant threshold.
    edit: Option<Box<EntryForm>>,
    last_activity: Instant,
    pub(crate) groups: Vec<Vec<String>>,
    pub(crate) page: Page,
    pub(crate) notice: Option<String>,
}

impl Vault {
    pub(super) fn new(
        master: MasterKey,
        keyfile: Option<PathBuf>,
        entries: Vec<EntrySummary>,
        groups: Vec<Vec<String>>,
        now: Instant,
    ) -> Self {
        Self {
            master,
            keyfile,
            matches: (0..entries.len()).collect(),
            entries,
            filter: String::new(),
            selected: 0,
            revealed: None,
            edit: None,
            last_activity: now,
            groups,
            page: Page::Browse(BrowsePage::Tree),
            notice: None,
        }
    }

    // ── Mutations ───────────────────────────────────────────────────────

    /// Record user activity, postponing [`super::IDLE_LOCK`].
    pub(super) fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Move the cursor, clamped at both ends — deliberately **without**
    /// wrap-around: Far Manager parks `PgDn` on the last row instead of
    /// jumping back to the top, and wrapping past the end of a password list
    /// makes it easy to copy the wrong secret.
    pub fn move_selection(&mut self, motion: Motion, now: Instant) {
        self.touch(now);
        self.hide();
        let Some(last) = self.matches.len().checked_sub(1) else {
            self.selected = 0;
            return;
        };
        self.selected = match motion {
            Motion::Prev => self.selected.saturating_sub(1),
            Motion::Next => self.selected.saturating_add(1).min(last),
            Motion::PageUp(rows) => self.selected.saturating_sub(rows),
            Motion::PageDown(rows) => self.selected.saturating_add(rows).min(last),
        };
    }

    /// Swap in a fresh summary list after a save changed the file.
    ///
    /// Goes through [`Self::refilter`] so `matches` and the cursor clamp are
    /// rebuilt in the single place they always are.
    pub(super) fn replace_catalog(
        &mut self,
        entries: Vec<EntrySummary>,
        groups: Vec<Vec<String>>,
        now: Instant,
    ) {
        self.entries = entries;
        self.groups = groups;
        self.refilter(now);
    }

    /// Park the cursor on the entry `id`.
    ///
    /// By identity, never by index: `keepass::list_entries` returns entries in
    /// an unstable order, so the row an entry had before a save says nothing
    /// about where it is afterwards. An entry the current filter hides is left
    /// alone — the clamp in [`Self::refilter`] already chose a valid row.
    pub(super) fn select_entry_by_id(&mut self, id: EntryId) {
        let found = self
            .matches
            .iter()
            .position(|&index| self.entries.get(index).is_some_and(|entry| entry.id == id));
        if let Some(position) = found {
            self.selected = position;
        }
    }
}
