//! Purpose: writes from a `KdbxSession` back to the file — confirmed delete and
//! undo here, the entry form's save in `persist/form.rs`.

use std::time::Instant;

use super::{Access, KdbxError, KdbxSession, UndoState};
use crate::keepass;

mod form;

impl KdbxSession {
    pub fn request_delete(&mut self, now: Instant) -> Result<(), KdbxError> {
        if self.access == Access::View {
            return Err(KdbxError::ReadOnly);
        }
        let vault = self.vault_mut().ok_or(KdbxError::Locked)?;
        if vault.selected_entry().is_none() {
            return Err(KdbxError::NoSelection);
        }
        vault.touch(now);
        vault.request_delete();
        Ok(())
    }

    pub fn confirm_delete(&mut self, now: Instant) -> Result<(), KdbxError> {
        if self.access == Access::View {
            return Err(KdbxError::ReadOnly);
        }
        let result = {
            let vault = self.vault().ok_or(KdbxError::Locked)?;
            let id = vault.selected_entry().ok_or(KdbxError::NoSelection)?.id;
            keepass::delete_entry_by_id(&self.path, vault.master().expose(), vault.keyfile(), id)
        };
        result.map_err(KdbxError::Keepass)?;
        self.undo = UndoState::Available;
        self.reload_catalog(now)?;
        if let Some(vault) = self.vault_mut() {
            vault.close_dialog();
        }
        Ok(())
    }

    pub fn undo_last_change(&mut self, now: Instant) -> Result<(), KdbxError> {
        if self.undo == UndoState::Unavailable {
            return Err(KdbxError::NothingToUndo);
        }
        keepass::restore_backup(&self.path).map_err(KdbxError::Keepass)?;
        self.reload_catalog(now)?;
        self.undo = UndoState::Unavailable;
        Ok(())
    }

    fn reload_catalog(&mut self, now: Instant) -> Result<(), KdbxError> {
        let catalog = {
            let vault = self.vault().ok_or(KdbxError::Locked)?;
            keepass::read_catalog(&self.path, vault.master().expose(), vault.keyfile())
        }
        .map_err(KdbxError::Keepass)?;
        let vault = self.vault_mut().ok_or(KdbxError::Locked)?;
        vault.replace_catalog(catalog.entries, catalog.groups, now);
        Ok(())
    }
}
