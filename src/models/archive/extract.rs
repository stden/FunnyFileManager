//! Purpose: `Shift+F2`'s Extract files dialog — where the selected archives unpack to, and what it says when none is selected.

use std::path::{Path, PathBuf};

use super::ARCHIVE_SUFFIXES;
use crate::lineedit::LineEditor;

/// Which control of the Extract files dialog carries the cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExtractField {
    /// The destination field, where the dialog opens.
    #[default]
    Destination,
    /// The default button.
    Extract,
    Cancel,
}

impl ExtractField {
    /// Tab order, top to bottom.
    const ORDER: [Self; 3] = [Self::Destination, Self::Extract, Self::Cancel];
}

/// `Shift+F2`: where the selected archives unpack to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractPrompt {
    /// The destination, offered as the other panel's folder. Free text: a
    /// relative path is taken from the active panel, and a folder that does
    /// not exist yet is created.
    pub destination: LineEditor,
    /// The selected entries that are archives, absolute. Empty when nothing
    /// selected is one — the dialog still opens and says so, because a key
    /// that visibly does nothing reads as a dead key.
    pub archives: Vec<PathBuf>,
    pub field: ExtractField,
}

impl ExtractPrompt {
    /// Open the dialog for `archives`, offering `destination`.
    #[must_use]
    pub fn new(archives: Vec<PathBuf>, destination: &Path) -> Self {
        Self {
            destination: LineEditor::from(crate::path_display::label(destination).into_owned()),
            archives,
            field: ExtractField::default(),
        }
    }

    /// There is something to extract, so the Extract button applies.
    #[must_use]
    pub fn can_extract(&self) -> bool {
        !self.archives.is_empty()
    }

    /// The dialog's first line: what is going to be unpacked.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.archives.as_slice() {
            [] => format!("No archive selected ({})", ARCHIVE_SUFFIXES.join(" ")),
            [one] => format!(
                "Extract \"{}\" to:",
                one.file_name().unwrap_or_default().to_string_lossy()
            ),
            many => format!("Extract {} archives to:", many.len()),
        }
    }

    #[must_use]
    pub fn on_cancel(&self) -> bool {
        self.field == ExtractField::Cancel
    }

    /// Move the cursor to the next control (or the previous one), skipping
    /// the Extract button while it is dimmed, as Far skips a disabled control.
    pub fn advance(&mut self, forward: bool) {
        self.field = crate::cycle::step(&ExtractField::ORDER, self.field, forward);
        if self.field == ExtractField::Extract && !self.can_extract() {
            self.field = crate::cycle::step(&ExtractField::ORDER, self.field, forward);
        }
    }

    pub fn push(&mut self, ch: char) {
        self.destination.insert_char(ch);
    }

    pub fn backspace(&mut self) {
        self.destination.backspace();
    }
}
