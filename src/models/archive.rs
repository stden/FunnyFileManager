//! Purpose: The three archive dialogs (`Shift+F1`, `Shift+F2`, `Shift+F3`), which share one model file because they share one set of archive operations.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

mod add;
mod commands;
mod extract;
pub use add::{ArchiveField, ArchivePrompt};
pub use commands::{ArchiveCommand, ArchiveMenu};
pub use extract::{ExtractField, ExtractPrompt};

/// The archive kinds the dialogs read: what `Shift+F2` unpacks, what the
/// `Shift+F3` rows accept, and what the captions list when nothing under the
/// cursor is one.
pub const ARCHIVE_SUFFIXES: [&str; 6] = [".tar", ".tar.gz", ".tgz", ".tar.xz", ".tar.bz2", ".zip"];

/// How an archive is packed: which tool runs and what the name ends in.
///
/// Three rather than Far's plugin list, because these are the three every
/// Linux, macOS and Windows box can be expected to read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    TarGz,
    Tar,
    Zip,
}

impl ArchiveFormat {
    /// Every format, in the order the radio shows them.
    pub const ALL: [Self; 3] = [Self::TarGz, Self::Tar, Self::Zip];

    /// What an archive of this format is named after the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Tar => "tar",
            Self::Zip => "zip",
        }
    }

    /// The radio label, with its hotkey marked.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TarGz => "tar.&gz",
            Self::Tar => "&tar",
            Self::Zip => "&zip",
        }
    }

    /// The program that writes this format.
    #[must_use]
    pub const fn tool(self) -> &'static str {
        match self {
            Self::TarGz | Self::Tar => "tar",
            Self::Zip => "zip",
        }
    }

    /// The format an existing archive's name says it has.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|format| lower.ends_with(&format!(".{}", format.extension())))
    }

    /// Whether files can be added to an existing archive of this format:
    /// `tar -r` appends to a plain tar and `zip` updates in place, but a
    /// compressed tar is one stream that cannot be extended.
    #[must_use]
    pub const fn appendable(self) -> bool {
        matches!(self, Self::Tar | Self::Zip)
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extract;
