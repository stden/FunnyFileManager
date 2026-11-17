//! Purpose: The Language servers form's controls: their tab order, the rows
//! they are drawn on, their help, and which of them offer suggestions.

use crate::complete::servers::CompletedField;

use super::FormTarget;

/// A control of the form, in tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Language,
    Command,
    Arguments,
    Extensions,
    Enabled,
    Ok,
    Cancel,
}

impl FormField {
    pub const ORDER: [Self; 7] = [
        Self::Language,
        Self::Command,
        Self::Arguments,
        Self::Extensions,
        Self::Enabled,
        Self::Ok,
        Self::Cancel,
    ];

    /// The controls of the form's body, top to bottom, as `ui` draws them:
    /// the rule and the help row follow, and the buttons have their own row.
    pub const ROWS: [Self; 5] = [
        Self::Language,
        Self::Command,
        Self::Arguments,
        Self::Extensions,
        Self::Enabled,
    ];

    /// The control on body row `row`, which a click reaches; `None` for the
    /// rule and the help row.
    #[must_use]
    pub fn at_row(row: usize) -> Option<Self> {
        Self::ROWS.get(row).copied()
    }

    /// Whether the control takes the cursor for `target`: an existing
    /// server's language is shown but cannot be renamed.
    #[must_use]
    pub fn allows(self, target: &FormTarget) -> bool {
        !(self == Self::Language && matches!(target, FormTarget::Existing(_)))
    }

    #[must_use]
    pub const fn is_text(self) -> bool {
        matches!(
            self,
            Self::Language | Self::Command | Self::Arguments | Self::Extensions
        )
    }

    /// The suggestions this control offers, if it offers any.
    #[must_use]
    pub const fn completed(self) -> Option<CompletedField> {
        match self {
            Self::Command => Some(CompletedField::Command),
            Self::Arguments => Some(CompletedField::Arguments),
            Self::Extensions => Some(CompletedField::Extensions),
            Self::Language | Self::Enabled | Self::Ok | Self::Cancel => None,
        }
    }

    /// The help row while this control has the cursor.
    #[must_use]
    pub const fn help(self) -> &'static str {
        match self {
            Self::Language => "The LSP languageId: a-z 0-9 - _ . +",
            Self::Command => "Program name on PATH, or a full path. No shell runs it.",
            Self::Arguments => "Separate with spaces; quote one that holds a space: \"a b\"",
            Self::Extensions => "Space-separated; the dot is optional: py pyi",
            Self::Enabled => "Space toggles",
            Self::Ok => "Enter keeps these values in the list; F2 there saves them.",
            Self::Cancel => "Enter drops the changes made in this form.",
        }
    }
}

/// A move through a list or the tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Previous,
}
