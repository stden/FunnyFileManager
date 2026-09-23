//! Purpose: whether the dialog serves F5 Copy or F6 Rename/Move, and which controls each allows.

use super::CopyField;

/// Whether the transfer keeps the original.
///
/// One dialog serves both keys: F5 and F6 differ in their title, their button
/// and which controls apply, and in nothing else. Two copies of the same
/// dialog would have to be kept in step by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Purpose {
    #[default]
    Copy,
    /// F6. Far titles it "Rename/Move", because moving within a directory is
    /// exactly a rename and the dialog is how you do either.
    Move,
}

impl Purpose {
    /// The dialog's title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Move => "Rename/Move",
        }
    }

    /// The label on the button that does it, with Far's hotkey marker.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Copy => "&Copy",
            Self::Move => "&Rename",
        }
    }

    /// What the status line says was done: `Copied to …`, `Moved 3 item(s)`.
    #[must_use]
    pub const fn done(self) -> &'static str {
        match self {
            Self::Copy => "Copied",
            Self::Move => "Moved",
        }
    }

    /// How the dialog's first line begins.
    #[must_use]
    pub const fn question(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Move => "Rename or move",
        }
    }

    /// Whether a control applies to this transfer.
    ///
    /// A move has one destination and no filter — Far draws both greyed rather
    /// than hiding them, so the dialog keeps its shape between the two keys.
    #[must_use]
    pub const fn allows(self, field: CopyField) -> bool {
        match field {
            // Following a link on a move would delete what the user had and
            // leave a copy of somewhere else in its place. It is a copying
            // choice, so a move greys it out like the other two.
            CopyField::MultipleDestinations | CopyField::Filter | CopyField::SymlinkContents => {
                matches!(self, Self::Copy)
            }
            _ => true,
        }
    }
}
