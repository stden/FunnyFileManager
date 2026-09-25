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
///
/// None of them can give a copy more than the source had or more than the
/// umask allows: a mode is only ever the source's, or the one the kernel
/// gave the file when the copy created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessRights {
    /// Whatever the platform's own copy does: a file keeps its source's
    /// permission bits, as with `cp` and `fs::copy`, and a folder is created
    /// like any new folder there.
    #[default]
    Default,
    /// The source's mode bits, carried over — onto every folder the copy
    /// makes as well as onto every file.
    Copy,
    /// What a file or folder newly created in the destination gets: the
    /// umask (or the folder's default ACL), applied by the kernel as each one
    /// is created, for everything the copy creates.
    Inherit,
}

impl AccessRights {
    /// The permission bits each copied file is given as it is written.
    #[must_use]
    pub const fn file_mode(self) -> crate::utils::FileMode {
        match self {
            Self::Default | Self::Copy => crate::utils::FileMode::Source,
            Self::Inherit => crate::utils::FileMode::Fresh,
        }
    }

    /// The permission bits each copied folder ends up with, restated once
    /// its contents are in. Only Copy carries the source's over; the others
    /// leave a folder as the kernel made it, the way `cp -r` does.
    #[must_use]
    pub const fn folder_mode(self) -> crate::utils::FileMode {
        match self {
            Self::Copy => crate::utils::FileMode::Source,
            Self::Default | Self::Inherit => crate::utils::FileMode::Fresh,
        }
    }

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
