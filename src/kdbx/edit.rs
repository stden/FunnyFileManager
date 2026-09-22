//! Purpose: the entry edit form of an unlocked `.kdbx` session (`F4`).
//!
//! Pure state, like [`super::vault`]: no rendering, no key types. The form
//! lives inside [`super::Vault`], which exists only in
//! `KdbxState::Unlocked`, so "editing while locked" is not representable.
//!
//! The draft password sits in a [`MasterKey`] rather than a `String`, so a
//! half-edited secret is zeroized when the form is cancelled, locked away, or
//! dropped — and so [`EntryForm`] cannot derive `Debug`, which keeps the whole
//! session unformattable (`tests_secrets.rs` asserts this structurally).

use super::secret::MasterKey;
use crate::keepass::EntryDetails;
use crate::keepass::EntryId;

/// Which form field has the cursor.
///
/// An enum rather than an index, for the reason `crate::models::NameField` is
/// one: the tab order is stated once, and a field added in the middle cannot
/// silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Title,
    Username,
    Url,
    Notes,
    Password,
}

impl EditField {
    /// The tab order, top to bottom.
    const ORDER: [Self; 5] = [
        Self::Title,
        Self::Username,
        Self::Url,
        Self::Notes,
        Self::Password,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn prev(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }
}

/// Whether the form is taking edits or asking whether to discard them.
///
/// A mode instead of a bool named `confirming`: `Esc` on a dirty form must not
/// throw the draft away silently, and the renderer and the key handler have to
/// agree on which question is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Editing,
    ConfirmSave,
}

enum EditValue {
    Text(String),
    Password(MasterKey),
}

struct EditUndo {
    field: EditField,
    value: EditValue,
    dirty: bool,
}

/// One entry being edited: the draft fields plus the identity they came from.
///
/// `origin_id` is what `save_edit` addresses, so a changed title renames this
/// entry in place instead of upserting a sibling — and a title shared with
/// another entry cannot make the save land on that one. `origin_group` and
/// `origin_title` are kept for the collision check and the dialog caption.
///
/// **No `Debug`**: [`MasterKey`] has none, which makes deriving one here a
/// compile error. That is the guarantee, not an oversight.
pub struct EntryForm {
    origin_group: Vec<String>,
    origin_title: String,
    origin_id: EntryId,
    title: String,
    username: String,
    url: String,
    notes: String,
    /// The draft password: a [`MasterKey`] rather than a `String`, so it is
    /// zeroized on drop and cannot be formatted.
    password: MasterKey,
    focus: EditField,
    mode: FormMode,
    dirty: bool,
    error: Option<String>,
    undo: Option<EditUndo>,
}

impl EntryForm {
    /// Build the form from a just-read entry.
    ///
    /// `pub(super)` so a form can only come from
    /// [`super::KdbxSession::begin_edit`] — the one caller allowed to see an
    /// [`EntryDetails`], whose password this constructor moves straight into
    /// the zeroizing wrapper.
    pub(super) fn new(
        origin_group: Vec<String>,
        origin_title: String,
        origin_id: EntryId,
        details: EntryDetails,
    ) -> Self {
        Self {
            origin_group,
            origin_title,
            origin_id,
            title: details.title,
            username: details.username,
            url: details.url,
            notes: details.notes,
            password: MasterKey::new(details.password),
            focus: EditField::Title,
            mode: FormMode::Editing,
            dirty: false,
            error: None,
            undo: None,
        }
    }

    // ── Read-only accessors for the renderer ────────────────────────────

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }

    /// The title the entry had when the form opened — the dialog caption.
    pub fn origin_title(&self) -> &str {
        &self.origin_title
    }

    /// The entry the save addresses.
    pub fn origin_id(&self) -> EntryId {
        self.origin_id
    }

    /// Group names from the root down to the entry's group; structural, not
    /// joined, for the reason `EntrySummary::group_path` is.
    pub fn origin_group(&self) -> &[String] {
        &self.origin_group
    }

    pub fn focus(&self) -> EditField {
        self.focus
    }

    pub fn mode(&self) -> FormMode {
        self.mode
    }

    /// Whether any keystroke changed the draft — what turns `Esc` into a
    /// discard question instead of a silent close.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Why the previous save failed, ready to display.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// What the renderer draws for the password: one bullet per character,
    /// never the text.
    pub fn password_masked(&self) -> String {
        "•".repeat(self.password.char_count())
    }

    /// The draft password in clear for the explicit edit dialog.
    pub fn password_visible(&self) -> &str {
        self.password.expose()
    }

    // ── Mutations ───────────────────────────────────────────────────────

    /// Route one typed character into the focused field.
    pub fn push_char(&mut self, ch: char) {
        // Every field is single-line in v1 — Notes included — and a stored
        // newline would render as a second list row. Refuse the character
        // rather than save it.
        if ch == '\n' || ch == '\r' {
            return;
        }
        // A new keystroke means the user is retrying; stop showing the stale
        // save failure next to fresh input.
        self.error = None;
        self.remember_undo();
        self.dirty = true;
        match self.focused_text_mut() {
            Some(text) => text.push(ch),
            None => self.password.push(ch),
        }
    }

    /// Backspace in the focused field; a no-op when the field is empty.
    pub fn backspace(&mut self) {
        self.error = None;
        self.remember_undo();
        if self.remove_last_char() {
            self.dirty = true;
        } else {
            self.undo = None;
        }
    }

    /// Remove the focused field's last character; whether one was there —
    /// deleting from an empty field must not mark the draft dirty.
    fn remove_last_char(&mut self) -> bool {
        if let Some(text) = self.focused_text_mut() {
            return text.pop().is_some();
        }
        if self.password.is_empty() {
            return false;
        }
        self.password.pop();
        true
    }

    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
    }

    pub fn set_mode(&mut self, mode: FormMode) {
        self.mode = mode;
    }

    /// Undo the most recent text mutation in this form.
    pub fn undo(&mut self) -> bool {
        let Some(undo) = self.undo.take() else {
            return false;
        };
        self.focus = undo.field;
        match undo.value {
            EditValue::Text(value) => {
                let Some(text) = self.focused_text_mut() else {
                    return false;
                };
                *text = value;
            }
            EditValue::Password(value) => self.password = value,
        }
        self.error = None;
        self.dirty = undo.dirty;
        true
    }

    // ── For `super` only ────────────────────────────────────────────────

    /// The draft password for the save path, revealed or not. `pub(super)`
    /// for the reason `Vault::master` is: only `super`'s writes may read it.
    pub(super) fn password(&self) -> &MasterKey {
        &self.password
    }

    pub(super) fn password_text(&self) -> String {
        self.password.expose().to_owned()
    }

    /// Record a failed save on the form, so it is shown next to the fields
    /// that caused it and cleared by the next keystroke.
    pub(super) fn set_error(&mut self, message: String) {
        self.error = Some(message);
    }

    /// Overwrite the draft secret now rather than waiting for `Drop`.
    pub(super) fn zeroize(&mut self) {
        self.password.zeroize();
    }

    /// The focused field's text, or `None` when the password has the cursor —
    /// its bytes live in a [`MasterKey`], not a `String`.
    fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            EditField::Title => Some(&mut self.title),
            EditField::Username => Some(&mut self.username),
            EditField::Url => Some(&mut self.url),
            EditField::Notes => Some(&mut self.notes),
            EditField::Password => None,
        }
    }

    fn remember_undo(&mut self) {
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
