//! Purpose: the *unlocked* half of a `.kdbx` session — the secret-free entry
//! list, the incremental filter over it, cursor movement, and the revealed
//! password.
//!
//! Pure state: no rendering, no key types, no clock of its own. Every mutating
//! method takes the caller's `Instant` so the idle auto-lock in
//! [`super::KdbxSession::tick`] is driven by real user activity and stays
//! testable without a terminal.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::edit::EntryForm;
use super::secret::{MasterKey, Revealed};
use crate::casefold::contains_ignore_case;
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

    // ── Read-only accessors for the renderer ────────────────────────────

    pub fn entries(&self) -> &[EntrySummary] {
        &self.entries
    }

    /// Indices into [`Self::entries`] that currently pass the filter.
    pub fn matches(&self) -> &[usize] {
        &self.matches
    }

    /// The entries to draw, in display order.
    pub fn visible(&self) -> impl Iterator<Item = &EntrySummary> {
        self.matches
            .iter()
            .filter_map(|&index| self.entries.get(index))
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Cursor position **within [`Self::matches`]**, always `< matches.len()`
    /// whenever any entry matches.
    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_entry(&self) -> Option<&EntrySummary> {
        self.entries.get(*self.matches.get(self.selected)?)
    }

    /// The revealed password, or `None` while it is masked.
    pub fn revealed(&self) -> Option<&str> {
        self.revealed.as_ref().map(Revealed::expose)
    }

    pub fn is_revealed(&self) -> bool {
        self.revealed.is_some()
    }

    /// The open edit form, or `None` while the user is browsing.
    pub fn form(&self) -> Option<&EntryForm> {
        self.edit.as_deref()
    }

    pub fn form_mut(&mut self) -> Option<&mut EntryForm> {
        self.edit.as_deref_mut()
    }

    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    pub(crate) fn groups(&self) -> &[Vec<String>] {
        &self.groups
    }

    // ── Mutations ───────────────────────────────────────────────────────

    /// Record user activity, postponing [`super::IDLE_LOCK`].
    pub(super) fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Append one character to the filter.
    pub fn push_filter_char(&mut self, ch: char, now: Instant) {
        self.filter.push(ch);
        self.refilter(now);
    }

    /// Delete the last character of the filter.
    pub fn pop_filter_char(&mut self, now: Instant) {
        self.filter.pop();
        self.refilter(now);
    }

    /// Replace the filter outright.
    pub fn set_filter(&mut self, filter: &str, now: Instant) {
        self.filter.clear();
        self.filter.push_str(filter);
        self.refilter(now);
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

    /// Stop showing the revealed password and overwrite it.
    ///
    /// Zeroizing explicitly rather than just assigning `None` keeps the
    /// guarantee visible at the call site, so a later "simplification" to a
    /// plain `self.revealed = None` reads as the behaviour change it is.
    pub(super) fn hide(&mut self) {
        if let Some(revealed) = &mut self.revealed {
            revealed.zeroize();
        }
        self.revealed = None;
    }

    pub(super) fn set_revealed(&mut self, revealed: Revealed) {
        self.hide();
        self.revealed = Some(revealed);
    }

    /// Open an edit form, replacing (and zeroizing) any draft left behind.
    pub(super) fn set_edit(&mut self, form: EntryForm) {
        self.clear_edit();
        self.edit = Some(Box::new(form));
    }

    /// Drop the draft, overwriting its password first.
    ///
    /// Explicit for the reason [`Self::hide`] is: zeroizing before the
    /// assignment keeps the guarantee visible at the call site, so a later
    /// "simplification" to `self.edit = None` reads as the change it is.
    pub(super) fn clear_edit(&mut self) {
        if let Some(form) = &mut self.edit {
            form.zeroize();
        }
        self.edit = None;
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

    /// Overwrite every secret this vault holds, before it is dropped.
    pub(super) fn zeroize(&mut self) {
        self.master.zeroize();
        // The draft is wiped in place, not dropped: the caller may still hold
        // the form (the zeroization test does), and the lock that follows
        // drops the whole vault anyway.
        if let Some(form) = &mut self.edit {
            form.zeroize();
        }
        self.hide();
        self.close_dialog();
    }

    // ── Credentials, for `super`'s re-reads only ────────────────────────

    pub(super) fn master(&self) -> &MasterKey {
        &self.master
    }

    pub(super) fn keyfile(&self) -> Option<&Path> {
        self.keyfile.as_deref()
    }

    // ── Internals ───────────────────────────────────────────────────────

    /// Rebuild [`Self::matches`] and re-clamp the cursor.
    fn refilter(&mut self, now: Instant) {
        self.touch(now);
        // The revealed password belongs to whichever entry was selected when
        // `Ctrl+R` was pressed. A filter edit can move the cursor onto a
        // different entry, so keeping it would print entry A's password next to
        // entry B's name.
        self.hide();
        self.close_dialog();

        let needle = self.filter.to_lowercase();
        self.matches = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| matches_filter(entry, &needle))
            .map(|(index, _)| index)
            .collect();

        // `selected` indexes `matches`, and narrowing the filter can shrink
        // `matches` out from under it. An index that outlives the list it points
        // into has already shipped once in this codebase (panel reload), so it
        // is clamped here — the single place `matches` is rebuilt — instead of
        // at every read site. With no matches this parks at 0 and
        // `selected_entry()` correctly returns `None`.
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }
}

/// Case-insensitive substring test over the three secret-free fields.
///
/// `needle` must already be lowercased — [`Vault::refilter`] does it once per
/// edit rather than once per entry.
fn matches_filter(entry: &EntrySummary, needle: &str) -> bool {
    [&entry.title, &entry.username, &entry.url]
        .into_iter()
        .any(|field| contains_ignore_case(field, needle))
        || entry
            .group_path
            .iter()
            .any(|group| contains_ignore_case(group, needle))
}
