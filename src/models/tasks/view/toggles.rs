//! Purpose: The view's switches, each flipped by one key: flat list or tree,
//! command line or name in the last column, kernel threads shown or hidden.

use super::super::{ProcessKind, ProcessSample};

/// The table's layout: one row per process, or the parent tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Arrangement {
    #[default]
    Flat,
    Tree,
}

impl Arrangement {
    /// The other layout: `F5`, `t`, `Ctrl+T`.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Flat => Self::Tree,
            Self::Tree => Self::Flat,
        }
    }
}

/// What the last column shows: `F2` switches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CommandView {
    #[default]
    CommandLine,
    Name,
}

impl CommandView {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::CommandLine => Self::Name,
            Self::Name => Self::CommandLine,
        }
    }

    /// The text the last column shows for `sample`.
    #[must_use]
    pub fn text(self, sample: &ProcessSample) -> &str {
        match self {
            Self::CommandLine => &sample.command,
            Self::Name => &sample.name,
        }
    }
}

/// Whether kernel threads are listed: `K` switches. Hidden by default, since
/// on a busy host they bury the processes a user came to find.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KernelThreads {
    Shown,
    #[default]
    Hidden,
}

impl KernelThreads {
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Shown => Self::Hidden,
            Self::Hidden => Self::Shown,
        }
    }

    pub(super) const fn admits(self, kind: ProcessKind) -> bool {
        matches!(
            (self, kind),
            (Self::Shown, _) | (Self::Hidden, ProcessKind::User)
        )
    }
}
