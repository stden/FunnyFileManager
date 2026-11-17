//! Purpose: the secrets an unlocked vault holds — the revealed password, the
//! draft of the edit form and the credentials — and how each is overwritten.

use std::path::Path;

use super::Vault;
use crate::kdbx::edit::EntryForm;
use crate::kdbx::secret::{MasterKey, Revealed};

impl Vault {
    /// Stop showing the revealed password and overwrite it.
    ///
    /// Zeroizing explicitly rather than just assigning `None` keeps the
    /// guarantee visible at the call site, so a later "simplification" to a
    /// plain `self.revealed = None` reads as the behaviour change it is.
    pub(in crate::kdbx) fn hide(&mut self) {
        if let Some(revealed) = &mut self.revealed {
            revealed.zeroize();
        }
        self.revealed = None;
    }

    pub(in crate::kdbx) fn set_revealed(&mut self, revealed: Revealed) {
        self.hide();
        self.revealed = Some(revealed);
    }

    /// Open an edit form, replacing (and zeroizing) any draft left behind.
    pub(in crate::kdbx) fn set_edit(&mut self, form: EntryForm) {
        self.clear_edit();
        self.edit = Some(Box::new(form));
    }

    /// Drop the draft, overwriting its password first.
    ///
    /// Explicit for the reason [`Self::hide`] is: zeroizing before the
    /// assignment keeps the guarantee visible at the call site, so a later
    /// "simplification" to `self.edit = None` reads as the change it is.
    pub(in crate::kdbx) fn clear_edit(&mut self) {
        if let Some(form) = &mut self.edit {
            form.zeroize();
        }
        self.edit = None;
    }

    /// Overwrite every secret this vault holds, before it is dropped.
    pub(in crate::kdbx) fn zeroize(&mut self) {
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

    pub(in crate::kdbx) fn master(&self) -> &MasterKey {
        &self.master
    }

    pub(in crate::kdbx) fn keyfile(&self) -> Option<&Path> {
        self.keyfile.as_deref()
    }
}
