//! Purpose: key routing, cursor motion, reveal and clipboard reads of a `KdbxSession`.

use std::time::Instant;

use super::{KdbxError, KdbxSession, KdbxState, Motion, Revealed, Vault};
use crate::keepass::{self, EntryDetails};

impl KdbxSession {
    /// Route one printable character: the password while locked, the focused
    /// form field while the edit form is open, the entry filter otherwise.
    ///
    /// A single entry point on purpose — separate ones would eventually let a
    /// master password be typed into the (rendered in clear) filter box.
    pub fn type_char(&mut self, ch: char, now: Instant) {
        match &mut self.state {
            KdbxState::Locked(prompt) => prompt.push_char(ch),
            KdbxState::Unlocked(vault) => {
                // Touched here because form edits bypass `push_filter_char`,
                // and an idle lock must not fire mid-keystroke.
                vault.touch(now);
                match vault.form_mut() {
                    Some(form) => form.push_char(ch),
                    None => vault.push_filter_char(ch, now),
                }
            }
        }
    }

    /// Backspace, routed the same way as [`Self::type_char`].
    pub fn backspace(&mut self, now: Instant) {
        match &mut self.state {
            KdbxState::Locked(prompt) => prompt.pop_char(),
            KdbxState::Unlocked(vault) => {
                vault.touch(now);
                match vault.form_mut() {
                    Some(form) => form.backspace(),
                    None => vault.pop_filter_char(now),
                }
            }
        }
    }

    /// Move the cursor over the filtered list; a no-op while locked.
    pub fn move_selection(&mut self, motion: Motion, now: Instant) {
        if let Some(vault) = self.vault_mut() {
            vault.move_selection(motion, now);
        }
    }

    /// Open or close the selected entry's password dialog.
    ///
    /// Revealing fetches on demand: nothing keeps an entry password in memory
    /// until the user explicitly asks for it.
    pub fn reveal(&mut self, now: Instant) -> Result<bool, KdbxError> {
        if let Some(vault) = self.vault_mut() {
            vault.touch(now);
            if vault.is_revealed() {
                vault.hide();
                vault.close_dialog();
                return Ok(false);
            }
        }
        let details = self.read_selected_entry()?;
        let vault = self.vault_mut().ok_or(KdbxError::Locked)?;
        vault.set_revealed(Revealed::new(details.password));
        vault.open_password();
        Ok(true)
    }

    pub fn close_vault_dialog(&mut self, now: Instant) {
        if let Some(vault) = self.vault_mut() {
            vault.touch(now);
            vault.hide();
            vault.close_dialog();
        }
    }

    /// The selected entry's password, for the caller to put on the clipboard.
    ///
    /// Re-reads the database instead of requiring a prior [`Self::reveal`], so
    /// copying never forces the user to put the secret on screen first. It is
    /// returned, never printed or logged — the caller owns it and should drop it
    /// as soon as the clipboard has it.
    pub fn copy_password(&mut self, now: Instant) -> Result<String, KdbxError> {
        let details = self.read_selected_entry()?;
        self.touch(now);
        Ok(details.password)
    }

    /// The selected entry's user name.
    ///
    /// Takes `&self` because it reads only the secret-free summary list; the key
    /// handler must call [`Self::touch`] to register the activity.
    pub fn copy_username(&self) -> Result<String, KdbxError> {
        let vault = self.vault().ok_or(KdbxError::Locked)?;
        let entry = vault.selected_entry().ok_or(KdbxError::NoSelection)?;
        Ok(entry.username.clone())
    }

    pub fn copy_edit_password(&mut self, now: Instant) -> Result<String, KdbxError> {
        let password = self
            .vault()
            .and_then(Vault::form)
            .ok_or(KdbxError::NoEdit)?
            .password_text();
        self.touch(now);
        Ok(password)
    }

    /// Re-open the database and read the selected entry in full.
    ///
    /// Private, and deliberately so: the returned [`EntryDetails`] *does* derive
    /// `Debug` and holds a plaintext password, so it must not escape this module
    /// except as the single `String` the caller asked for.
    pub(super) fn read_selected_entry(&self) -> Result<EntryDetails, KdbxError> {
        let vault = self.vault().ok_or(KdbxError::Locked)?;
        let entry = vault.selected_entry().ok_or(KdbxError::NoSelection)?;
        // By id, never by title: two entries may share one, and the row the
        // user is looking at must be the one whose password they get.
        keepass::read_entry_by_id(
            &self.path,
            vault.master().expose(),
            vault.keyfile(),
            entry.id,
        )
        .map_err(KdbxError::Keepass)
    }
}
