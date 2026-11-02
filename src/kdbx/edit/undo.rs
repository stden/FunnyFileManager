//! Purpose: the one-step undo of the entry form's most recent text mutation.

use super::{EditField, EntryForm};
use crate::kdbx::secret::MasterKey;

enum EditValue {
    Text(String),
    Password(MasterKey),
}

pub(super) struct EditUndo {
    field: EditField,
    value: EditValue,
    dirty: bool,
}

impl Drop for EditUndo {
    fn drop(&mut self) {
        if let EditValue::Text(ref mut text) = self.value {
            crate::kdbx::secret::zeroize_string(text);
        }
    }
}

impl EntryForm {
    /// Undo the most recent text mutation in this form.
    pub fn undo(&mut self) -> bool {
        let Some(undo) = self.undo.take() else {
            return false;
        };
        self.focus = undo.field;
        match undo.value {
            EditValue::Text(ref value) => {
                let Some(text) = self.focused_text_mut() else {
                    return false;
                };
                text.clone_from(value);
            }
            EditValue::Password(ref value) => {
                self.password = MasterKey::new(value.expose().to_owned());
            }
        }
        self.error = None;
        self.dirty = undo.dirty;
        true
    }

    pub(super) fn remember_undo(&mut self) {
        let value = if self.focus == EditField::Password {
            EditValue::Password(MasterKey::new(self.password.expose().to_owned()))
        } else {
            let Some(text) = self.focused_text_mut() else {
                return;
            };
            EditValue::Text(text.clone())
        };
        self.undo = Some(EditUndo {
            field: self.focus,
            value,
            dirty: self.dirty,
        });
    }
}
