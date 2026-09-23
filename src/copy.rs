//! Purpose: the state behind Far's F5 Copy dialog.
//!
//! F5 used to copy to the other panel the instant it was pressed. That is the
//! common case and it is also the one case where a wrong keypress is expensive:
//! there is no undo for a copy that overwrites, and no way to say "somewhere
//! else" without first navigating the other panel there.
//!
//! So the key opens a dialog instead, and this module is what it edits. Kept
//! apart from the renderer and from the file operations: the dialog's rules —
//! which control the cursor is on, which options apply — are worth testing
//! without a terminal or a filesystem, and neither of those can be arranged
//! inside a `#[test]` as cheaply as calling a function.

use std::path::Path;

mod field;
mod options;
mod purpose;

pub use field::{field_at_row, CopyField};
pub use options::{AccessRights, ExistingFiles};
pub use purpose::Purpose;

/// Everything the Copy dialog is editing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the Copy dialog's independent Far checkboxes; every combination is a valid request"
)]
pub struct CopyPrompt {
    /// Where the files are going. Free text: it may be typed, and it may name
    /// somewhere neither panel is showing.
    pub destination: String,
    pub rights: AccessRights,
    pub existing: ExistingFiles,
    pub timestamps: bool,
    pub symlink_contents: bool,
    pub multiple_destinations: bool,
    pub filter: bool,
    pub field: CopyField,
    /// What is being copied, for the dialog's first line: a file name, or a
    /// count when several are marked.
    pub what: String,
    pub purpose: Purpose,
}

impl CopyPrompt {
    /// Open the dialog for `what`, offering `destination` as the default.
    #[must_use]
    pub fn new(what: impl Into<String>, destination: &Path) -> Self {
        Self::for_purpose(Purpose::Copy, what, destination)
    }

    /// The same dialog, for F5 or F6.
    #[must_use]
    pub fn for_purpose(purpose: Purpose, what: impl Into<String>, destination: &Path) -> Self {
        Self {
            purpose,
            destination: destination.to_string_lossy().into_owned(),
            rights: AccessRights::default(),
            existing: ExistingFiles::default(),
            timestamps: false,
            symlink_contents: false,
            multiple_destinations: false,
            filter: false,
            field: CopyField::default(),
            what: what.into(),
        }
    }

    /// The dialog's first line: `Copy "name" to:`.
    #[must_use]
    pub fn caption(&self) -> String {
        format!("{} {} t&o:", self.purpose.question(), self.what)
    }

    /// What the transfer does with a symbolic link it meets.
    ///
    /// The dialog's "Copy contents of symbolic links", which a move greys out
    /// — see [`Purpose::allows`] — so a move always recreates.
    #[must_use]
    pub const fn link_policy(&self) -> crate::utils::LinkPolicy {
        if self.symlink_contents && self.allows(CopyField::SymlinkContents) {
            crate::utils::LinkPolicy::Follow
        } else {
            crate::utils::LinkPolicy::Recreate
        }
    }

    /// How the transfer copies: what it does with a link it meets, and the
    /// permission bits each copied file is given.
    #[must_use]
    pub const fn rules(&self) -> crate::utils::CopyRules {
        crate::utils::CopyRules {
            links: self.link_policy(),
            modes: self.rights.file_mode(),
        }
    }

    /// Whether the control under the cursor is one this dialog offers: that
    /// the purpose allows it, and that something behind it carries it out.
    #[must_use]
    pub const fn allows(&self, field: CopyField) -> bool {
        field.is_wired() && self.purpose.allows(field)
    }

    /// Move to the next control the dialog actually offers.
    ///
    /// A greyed control is drawn but skipped: Far shows it so the dialog keeps
    /// its shape, and stopping the cursor on something that does nothing would
    /// undo the point of greying it.
    pub fn advance(&mut self, forward: bool) {
        for _ in 0..9 {
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

    /// Toggle whatever checkbox the cursor is on. Does nothing elsewhere, so
    /// the space bar stays a space in the destination field.
    pub fn toggle(&mut self) {
        if !self.allows(self.field) {
            return;
        }
        match self.field {
            CopyField::Timestamps => self.timestamps = !self.timestamps,
            CopyField::SymlinkContents => self.symlink_contents = !self.symlink_contents,
            CopyField::MultipleDestinations => {
                self.multiple_destinations = !self.multiple_destinations;
            }
            CopyField::Filter => self.filter = !self.filter,
            _ => {}
        }
    }

    /// Left/Right on a multiple-choice control: the radio group and the
    /// drop-down. Elsewhere the arrows belong to the text field.
    pub fn cycle(&mut self, forward: bool) {
        match self.field {
            CopyField::Rights => {
                self.rights = if forward {
                    self.rights.next()
                } else {
                    self.rights.previous()
                };
            }
            CopyField::Existing => {
                self.existing = if forward {
                    self.existing.next()
                } else {
                    self.existing.previous()
                };
            }
            _ => {}
        }
    }

    /// Whether the dialog can be accepted. An empty destination cannot: it
    /// would resolve to the working directory, which is not what an empty
    /// field means to the person who just cleared it.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.destination.trim().is_empty()
    }
}

#[cfg(test)]
mod tests;
