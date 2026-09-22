//! Purpose: the Copy dialog's multiple-choice options: what to do with existing files and which access rights the copy gets.

/// What to do when the destination already has a file of that name.
///
/// Far offers more (Rename, Append); these are the four whose behaviour the
/// copy underneath can actually carry out. Offering a fifth that silently did
/// the same thing as Overwrite would be worse than not offering it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExistingFiles {
    /// Stop and ask, once per collision.
    #[default]
    Ask,
    Overwrite,
    Skip,
    /// Overwrite only when the source is newer than what is there.
    OnlyNewer,
}

impl ExistingFiles {
    /// The label Far puts in the drop-down.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ask => "Ask",
            Self::Overwrite => "Overwrite",
            Self::Skip => "Skip",
            Self::OnlyNewer => "Only newer files",
        }
    }

    /// The next choice, for the arrow keys and the space bar.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Ask => Self::Overwrite,
            Self::Overwrite => Self::Skip,
            Self::Skip => Self::OnlyNewer,
            Self::OnlyNewer => Self::Ask,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Ask => Self::OnlyNewer,
            Self::Overwrite => Self::Ask,
            Self::Skip => Self::Overwrite,
            Self::OnlyNewer => Self::Skip,
        }
    }
}

/// What permissions the copy ends up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessRights {
    /// Whatever the platform's own copy does.
    #[default]
    Default,
    /// The source's mode bits, carried over.
    Copy,
    /// The destination directory's, as a newly created file would get.
    Inherit,
}

impl AccessRights {
    /// The three options in the order Far draws them.
    pub const ALL: [Self; 3] = [Self::Default, Self::Copy, Self::Inherit];

    /// Labels carrying Far's hotkey markers.
    pub const LABELS: [&'static str; 3] = ["Defau&lt", "Co&py", "&Inherit"];

    #[must_use]
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&it| it == self).unwrap_or(0)
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Default => Self::Copy,
            Self::Copy => Self::Inherit,
            Self::Inherit => Self::Default,
        }
    }

    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Default => Self::Inherit,
            Self::Copy => Self::Default,
            Self::Inherit => Self::Copy,
        }
    }
}
