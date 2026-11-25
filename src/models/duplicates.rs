//! Purpose: The Find duplicates dialog (Commands → Find duplicate blocks): where
//! to look, and how much text counts as a copy.
//!
//! State only: the scan lives in `ops::duplicates`, and the keys that move
//! through the dialog live in `popup`.

use crate::lineedit::LineEditor;

/// The block size the dialog starts with, in non-blank lines.
pub const DEFAULT_MIN_BLOCK: usize = 6;

/// How many groups the dialog starts with.
pub const DEFAULT_GROUPS: usize = 50;

/// Which control of the dialog has the cursor, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicatesField {
    #[default]
    Folder,
    Mask,
    MinBlock,
    Groups,
    Start,
    Cancel,
}

impl DuplicatesField {
    const ORDER: [Self; 6] = [
        Self::Folder,
        Self::Mask,
        Self::MinBlock,
        Self::Groups,
        Self::Start,
        Self::Cancel,
    ];

    /// The control after this one, or before it, wrapping at either end.
    fn step(self, forward: bool) -> Self {
        let count = Self::ORDER.len();
        let position = Self::ORDER
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        let next = if forward {
            (position + 1) % count
        } else {
            (position + count - 1) % count
        };
        Self::ORDER.get(next).copied().unwrap_or(self)
    }
}

/// The search as typed, before it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicates {
    /// The folder to search, with its subfolders.
    pub folder: LineEditor,
    /// The file names to compare: a mask such as `*.rs`.
    pub mask: LineEditor,
    /// The fewest non-blank lines a copy may be.
    pub min_block: LineEditor,
    /// How many groups the list shows.
    pub groups: LineEditor,
    field: DuplicatesField,
}

impl Duplicates {
    /// The dialog for a search of `folder`, with the defaults for the rest.
    #[must_use]
    pub fn new(folder: &str) -> Self {
        Self {
            folder: LineEditor::new(folder),
            mask: LineEditor::new("*"),
            min_block: LineEditor::new(&DEFAULT_MIN_BLOCK.to_string()),
            groups: LineEditor::new(&DEFAULT_GROUPS.to_string()),
            field: DuplicatesField::Folder,
        }
    }

    #[must_use]
    pub const fn field(&self) -> DuplicatesField {
        self.field
    }

    pub fn focus(&mut self, field: DuplicatesField) {
        self.field = field;
    }

    pub fn tab(&mut self, forward: bool) {
        self.field = self.field.step(forward);
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        matches!(self.field, DuplicatesField::Cancel)
    }

    #[must_use]
    pub const fn on_button(&self) -> bool {
        matches!(self.field, DuplicatesField::Start | DuplicatesField::Cancel)
    }

    pub fn toggle_button(&mut self) {
        self.field = match self.field {
            DuplicatesField::Start => DuplicatesField::Cancel,
            DuplicatesField::Cancel => DuplicatesField::Start,
            field => field,
        };
    }

    /// The text field the cursor is on, if it is on one.
    pub const fn active_line_mut(&mut self) -> Option<&mut LineEditor> {
        match self.field {
            DuplicatesField::Folder => Some(&mut self.folder),
            DuplicatesField::Mask => Some(&mut self.mask),
            DuplicatesField::MinBlock => Some(&mut self.min_block),
            DuplicatesField::Groups => Some(&mut self.groups),
            DuplicatesField::Start | DuplicatesField::Cancel => None,
        }
    }
}

#[cfg(test)]
mod tests;
