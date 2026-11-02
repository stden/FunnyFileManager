//! Purpose: the entry form's round trip — opened from the file with `F4`,
//! discarded, or written back to it.

use std::path::Path;
use std::time::Instant;

use crate::kdbx::{Access, EntryForm, KdbxError, KdbxSession, UndoState, Vault};
use crate::keepass::{self, EntryDetails, KeepassError};

impl KdbxSession {
    /// Open the edit form for the selected entry (`F4`).
    ///
    /// Reads the entry in full — password included — and moves it straight
    /// into an [`EntryForm`]; the intermediate [`EntryDetails`] never leaves
    /// this module (same rule as [`Self::read_selected_entry`]). The origin
    /// identity is taken from the summary so a later rename still addresses
    /// this entry.
    pub fn begin_edit(&mut self, now: Instant) -> Result<(), KdbxError> {
        if self.access == Access::View {
            return Err(KdbxError::ReadOnly);
        }
        let (origin_group, origin_title, origin_id) = {
            let vault = self.vault().ok_or(KdbxError::Locked)?;
            let entry = vault.selected_entry().ok_or(KdbxError::NoSelection)?;
            (entry.group_path.clone(), entry.title.clone(), entry.id)
        };
        let details = self.read_selected_entry()?;
        let vault = self.vault_mut().ok_or(KdbxError::Locked)?;
        vault.touch(now);
        vault.set_edit(EntryForm::new(
            origin_group,
            origin_title,
            origin_id,
            details,
        ));
        Ok(())
    }

    /// Close the form and throw the draft away (`Esc` → Discard).
    ///
    /// The draft password is zeroized on the way out (`Vault::clear_edit`);
    /// the entry on disk is untouched.
    pub fn cancel_edit(&mut self, now: Instant) {
        if let Some(vault) = self.vault_mut() {
            vault.touch(now);
            vault.clear_edit();
        }
    }

    /// Write the open form back to the file. Returns whether the save
    /// succeeded, mirroring [`Self::unlock`].
    ///
    /// On success the form closes and the summaries are re-read from the
    /// file; on failure the form stays open with a safe error line
    /// ([`save_error_message`]), so nothing the user typed is lost. The write
    /// itself is `keepass::update_entry`: addressed by the origin identity,
    /// through edit tracking, saved atomically with a `.bak` behind it.
    pub fn save_edit(&mut self, now: Instant) -> bool {
        if self.access == Access::View {
            return false;
        }
        let result = {
            let Some(vault) = self.vault() else {
                return false;
            };
            let Some(form) = vault.form() else {
                return false;
            };
            save_form(&self.path, vault, form)
        };
        self.touch(now);
        match result {
            Ok(()) => {
                self.finish_save(now);
                self.undo = UndoState::Available;
                true
            }
            Err(error) => {
                if let Some(form) = self.vault_mut().and_then(Vault::form_mut) {
                    form.set_error(save_error_message(&error));
                }
                false
            }
        }
    }

    /// After a successful write: close the form and re-derive the summaries,
    /// so the list shows what the file now says rather than a local patch.
    fn finish_save(&mut self, now: Instant) {
        let catalog = {
            let Some(vault) = self.vault() else {
                return;
            };
            keepass::read_catalog(&self.path, vault.master().expose(), vault.keyfile())
        };
        let Some(vault) = self.vault_mut() else {
            return;
        };
        let Some(form) = vault.form() else {
            return;
        };
        // The identity, taken before the form goes away. Reselecting by it —
        // never by index, because the listing order is unstable across reads,
        // and never by title, because a title can be shared.
        let id = form.origin_id();
        vault.clear_edit();
        if let Ok(catalog) = catalog {
            vault.replace_catalog(catalog.entries, catalog.groups, now);
            vault.select_entry_by_id(id);
        }
        // A failed re-list keeps the stale summaries: the save itself
        // succeeded, and reporting a failure here would invite a second
        // write. The next unlock re-reads the file anyway.
    }
}

/// Turn a failed save into a message that is safe to display.
///
/// Same contract as [`unlock_error_message`]: the string reaches the UI and
/// may sit near logging code, so nothing derived from the credentials or the
/// entry is allowed through. `Save` / `Io` texts are upstream or OS strings —
/// a kdbx3 vault surfaces the writer's unsupported-version text here.
fn save_error_message(error: &KeepassError) -> String {
    match error {
        KeepassError::EntryNotFound => "Entry no longer exists in the vault".to_owned(),
        KeepassError::AlreadyExists => "An entry with that title already exists".to_owned(),
        KeepassError::Io(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            "The vault file is read-only for you: nothing was saved".to_owned()
        }
        other => other.to_string(),
    }
}

fn save_form(path: &Path, vault: &Vault, form: &EntryForm) -> Result<(), KeepassError> {
    let group: Vec<&str> = form.origin_group().iter().map(String::as_str).collect();
    let details = EntryDetails {
        title: form.title().to_owned(),
        username: form.username().to_owned(),
        url: form.url().to_owned(),
        notes: form.notes().to_owned(),
        // The draft leaves its zeroizing wrapper only for this call;
        // `details` dies with the block.
        password: form.password().expose().to_owned(),
    };
    keepass::update_entry_by_id(
        path,
        vault.master().expose(),
        vault.keyfile(),
        &group,
        form.origin_id(),
        &details,
    )
}
