//! Purpose: The find-file dialog (`Alt+F7`): what to look for, and where.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

use crate::lineedit::LineEditor;

/// Which control of the Find file dialog has the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FindFileField {
    #[default]
    Mask,
    Containing,
    Find,
    Cancel,
}

/// The search as typed, before it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindFile {
    /// The name mask.
    pub mask: LineEditor,
    /// Far's "Containing text".
    pub containing: LineEditor,
    field: FindFileField,
}

impl Default for FindFile {
    fn default() -> Self {
        Self {
            mask: LineEditor::new("*"),
            containing: LineEditor::default(),
            field: FindFileField::Mask,
        }
    }
}

impl FindFile {
    #[must_use]
    pub const fn field(&self) -> FindFileField {
        self.field
    }

    pub fn tab(&mut self, forward: bool) {
        self.field = if forward {
            self.field.next()
        } else {
            self.field.previous()
        };
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        matches!(self.field, FindFileField::Cancel)
    }

    #[must_use]
    pub const fn on_button(&self) -> bool {
        matches!(self.field, FindFileField::Find | FindFileField::Cancel)
    }

    pub fn toggle_button(&mut self) {
        self.field = match self.field {
            FindFileField::Find => FindFileField::Cancel,
            FindFileField::Cancel => FindFileField::Find,
            field => field,
        };
    }

    pub fn push(&mut self, ch: char) {
        if let Some(line) = self.active_line_mut() {
            line.insert_char(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(line) = self.active_line_mut() {
            line.backspace();
        }
    }

    /// The text field the cursor is on, if it is on one.
    pub const fn active_line_mut(&mut self) -> Option<&mut LineEditor> {
        match self.field {
            FindFileField::Mask => Some(&mut self.mask),
            FindFileField::Containing => Some(&mut self.containing),
            FindFileField::Find | FindFileField::Cancel => None,
        }
    }
}

impl FindFileField {
    const fn next(self) -> Self {
        match self {
            Self::Mask => Self::Containing,
            Self::Containing => Self::Find,
            Self::Find => Self::Cancel,
            Self::Cancel => Self::Mask,
        }
    }

    const fn previous(self) -> Self {
        match self {
            Self::Mask => Self::Cancel,
            Self::Containing => Self::Mask,
            Self::Find => Self::Containing,
            Self::Cancel => Self::Find,
        }
    }
}

#[cfg(test)]
mod tests;
