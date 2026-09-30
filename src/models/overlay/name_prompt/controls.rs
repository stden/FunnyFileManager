//! Purpose: The name dialog's controls: which ones it offers, moving and
//! clicking between them, the checkbox, the link drop-down, and typing.

use super::{NameField, NamePrompt, NamePurpose};

impl NamePrompt {
    /// Whether this dialog offers that control at all.
    ///
    /// Shift+F6 is Far's plain Rename box: it asks for a name and nothing
    /// else. The link controls belong to F7, and the Target field only once
    /// there is a link for it to describe — greyed otherwise, so the dialog
    /// keeps its shape as the drop-down changes.
    #[must_use]
    pub const fn allows(&self, field: NameField) -> bool {
        match field {
            NameField::Name | NameField::Ok | NameField::Cancel => true,
            NameField::Link => self.purpose.offers_links(),
            NameField::Multiple => matches!(self.purpose, NamePurpose::MakeFolder),
            NameField::Target => self.purpose.offers_links() && self.link.needs_target(),
        }
    }

    /// Move to the next control the dialog actually offers, skipping the
    /// greyed ones: stopping on a control that does nothing would undo the
    /// point of greying it.
    pub fn advance(&mut self, forward: bool) {
        for _ in 0..NameField::ORDER.len() {
            self.field = if forward {
                self.field.next()
            } else {
                self.field.previous()
            };
            if self.allows(self.field) {
                return;
            }
        }
    }

    /// Put the cursor on `field`, if the dialog offers it. What a click means.
    pub const fn focus(&mut self, field: NameField) {
        if self.allows(field) {
            self.field = field;
        }
    }

    /// Toggle the checkbox under the cursor. Does nothing elsewhere, so the
    /// space bar stays a space in the two text fields.
    pub const fn toggle(&mut self) {
        if matches!(self.field, NameField::Multiple) && self.allows(NameField::Multiple) {
            self.multiple = !self.multiple;
        }
    }

    /// Left/Right on the drop-down. Elsewhere the arrows are not ours.
    pub fn cycle(&mut self, forward: bool) {
        if !matches!(self.field, NameField::Link) || !self.allows(NameField::Link) {
            return;
        }
        self.link = crate::cycle::step(self.purpose.link_choices(), self.link, forward);
        // The Target field may have just been greyed out from under the
        // cursor. It cannot be, in fact — the cursor is on the drop-down — but
        // the invariant is worth holding rather than reasoning about.
        if !self.allows(self.field) {
            self.field = NameField::Name;
        }
    }

    /// Which control the dialog draws on body row `row`.
    ///
    /// The row order lives here rather than only in the renderer, because the
    /// mouse needs it too: a click is only right if it reaches the control the
    /// user is pointing at.
    #[must_use]
    pub const fn field_at_row(&self, row: usize) -> Option<NameField> {
        // 0 is the caption, 2 is the rule under the name field.
        match (self.purpose, row) {
            (_, 1) => Some(NameField::Name),
            (purpose, 3) if purpose.offers_links() => Some(NameField::Link),
            (purpose, 4) if purpose.offers_links() => Some(NameField::Target),
            (NamePurpose::MakeFolder, 5) => Some(NameField::Multiple),
            _ => None,
        }
    }

    pub fn push(&mut self, ch: char) {
        if let Some(line) = self.focused_line_mut() {
            line.insert_char(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(line) = self.focused_line_mut() {
            line.backspace();
        }
    }
}
