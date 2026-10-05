//! Purpose: Where the editor's buffer stands relative to the file on disk,
//! and the questions (save on exit, overwrite a changed file) it asks.

use super::OverwritePrompt;

/// Where the buffer stands relative to the file on disk.
///
/// This replaced `modified: bool` beside `confirm_exit: bool`, which made
/// "asking whether to save, with nothing to save" a representable state. The
/// exit dialog exists *because* there are unsaved changes, so it is a stage of
/// being modified rather than an independent flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditState {
    /// Saved, or never touched.
    #[default]
    Clean,
    /// Edited since the last save.
    Modified,
    /// Esc was pressed with unsaved changes; awaiting save/discard/cancel.
    ///
    /// Carries which button the cursor is on. An `Option<usize>` index would
    /// have states naming no button at all, and every caller would then have to
    /// defend against them; three named answers cannot be out of range.
    ConfirmingExit(ExitChoice),
    /// A save found the file changed on disk since it was read; awaiting
    /// Overwrite or Cancel.
    ConfirmingOverwrite(OverwritePrompt),
}

/// The button focused in the editor's unsaved-changes dialog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExitChoice {
    /// Save and close — the default, which is what Enter starts on.
    #[default]
    Save,
    /// Close without saving.
    Discard,
    /// Stay in the editor.
    Cancel,
}

impl ExitChoice {
    /// The next button rightwards, wrapping — which is what Far's Right and
    /// Tab both do in a dialog.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Save => Self::Discard,
            Self::Discard => Self::Cancel,
            Self::Cancel => Self::Save,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Save => Self::Cancel,
            Self::Discard => Self::Save,
            Self::Cancel => Self::Discard,
        }
    }
}

impl EditState {
    /// Whether the buffer differs from the file — true in both edited stages.
    #[must_use]
    pub const fn is_modified(self) -> bool {
        matches!(
            self,
            Self::Modified | Self::ConfirmingExit(_) | Self::ConfirmingOverwrite(_)
        )
    }

    #[must_use]
    pub const fn is_confirming_exit(self) -> bool {
        matches!(self, Self::ConfirmingExit(_))
    }

    /// Which button the exit dialog has focus on, if it is open.
    #[must_use]
    pub const fn exit_choice(self) -> Option<ExitChoice> {
        match self {
            Self::ConfirmingExit(choice) => Some(choice),
            _ => None,
        }
    }

    /// The "changed on disk" question, if it is open.
    #[must_use]
    pub const fn overwrite_prompt(self) -> Option<OverwritePrompt> {
        match self {
            Self::ConfirmingOverwrite(prompt) => Some(prompt),
            _ => None,
        }
    }
}
