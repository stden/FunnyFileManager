//! Purpose: The cursor over the attributes dialog's controls: the tab ring,
//! Space on a box, typing into a time field, and a click on the buttons.

use super::AttributesDialog;

/// Which control has the cursor. The ring is computed from the platform's
/// flag count, so the same enum serves both layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributesField {
    /// The `index`-th box, in [`Flag::GROUPS`](super::Flag::GROUPS) order.
    Flag(usize),
    Modified,
    Accessed,
    Subfolders,
    Set,
    Cancel,
}

impl Default for AttributesField {
    /// The cursor opens on the first box, as Far's does.
    fn default() -> Self {
        Self::Flag(0)
    }
}

impl AttributesDialog {
    /// The tab ring, top to bottom: every box, the times, the subfolders box
    /// when it applies, then the buttons.
    fn ring(&self) -> Vec<AttributesField> {
        let mut ring: Vec<AttributesField> =
            (0..self.flags.len()).map(AttributesField::Flag).collect();
        ring.push(AttributesField::Modified);
        ring.push(AttributesField::Accessed);
        if self.has_directories {
            ring.push(AttributesField::Subfolders);
        }
        ring.push(AttributesField::Set);
        ring.push(AttributesField::Cancel);
        ring
    }

    /// Tab / Shift+Tab: the next or previous control, wrapping.
    pub fn advance(&mut self, forward: bool) {
        let ring = self.ring();
        self.field = crate::cycle::step(&ring, self.field, forward);
    }

    /// Space: flip the box under the cursor, if it is one.
    pub fn toggle(&mut self) {
        match self.field {
            AttributesField::Flag(index) => {
                if let Some(flag_box) = self.flags.get_mut(index) {
                    flag_box.toggle();
                }
            }
            AttributesField::Subfolders if self.has_directories => {
                self.process_subfolders = !self.process_subfolders;
            }
            _ => {}
        }
    }

    /// The time field under the cursor, if the cursor is on one.
    fn time_field_mut(&mut self) -> Option<&mut String> {
        match self.field {
            AttributesField::Modified => Some(&mut self.modified.current),
            AttributesField::Accessed => Some(&mut self.accessed.current),
            _ => None,
        }
    }

    /// A typed character, into the focused time field only.
    pub fn push(&mut self, ch: char) {
        if let Some(field) = self.time_field_mut() {
            field.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(field) = self.time_field_mut() {
            field.pop();
        }
    }

    /// A click on the button row: `0` is Set, anything else Cancel.
    pub fn focus_button(&mut self, index: usize) {
        self.field = if index == 0 {
            AttributesField::Set
        } else {
            AttributesField::Cancel
        };
    }
}
