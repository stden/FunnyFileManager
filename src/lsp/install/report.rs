//! Purpose: Say what FFM would start for a configured server, without starting
//! a process or installing a package.

use super::InstallError;
use std::path::PathBuf;

mod lookup;

pub use lookup::availability;
#[cfg(test)]
use lookup::{resolve, workspace};

/// A program FFM would run, and the arguments it would pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

/// Why nothing is installed to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotInstalled {
    AutoInstallOff,
    NoCache,
    AfterTrust,
}

/// The configured command was not found, and the auto-install recipe's
/// program runs instead of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replacement {
    Cached(Resolved),
    AfterTrust,
}

/// How a status reads at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Pending,
    Bad,
}

/// What FFM would start for one server — never a process list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// The configured command was found.
    Configured(Resolved),
    /// The recipe that supplies the configured command is installed.
    Cached(Resolved),
    NotInstalled(NotInstalled),
    Replaced(Replacement),
    /// A corrupt receipt, an I/O error, or a missing runtime (Java 21, Node.js).
    Failed(InstallError),
}

impl Availability {
    /// The Status row's words.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Configured(_) => "found configured executable",
            Self::Cached(_) => "installed in FFM cache",
            Self::NotInstalled(NotInstalled::AfterTrust) => {
                "not installed; installs after workspace trust"
            }
            Self::NotInstalled(NotInstalled::AutoInstallOff) => {
                "not installed; auto-install disabled"
            }
            Self::NotInstalled(NotInstalled::NoCache) => "no installation cache configured",
            Self::Replaced(Replacement::Cached(_)) => {
                "command not found; FFM runs the auto-installed server instead"
            }
            Self::Replaced(Replacement::AfterTrust) => {
                "command not found; FFM will install and run the auto-install server instead"
            }
            Self::Failed(error) => error.label(),
        }
    }

    #[must_use]
    pub const fn tone(&self) -> Tone {
        match self {
            Self::Configured(_) | Self::Cached(_) => Tone::Good,
            Self::NotInstalled(NotInstalled::AfterTrust) | Self::Replaced(_) => Tone::Pending,
            Self::NotInstalled(_) | Self::Failed(_) => Tone::Bad,
        }
    }

    /// The one word the server list shows.
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Configured(_) | Self::Cached(_) => "ready",
            Self::NotInstalled(NotInstalled::AfterTrust) => "auto",
            Self::Replaced(_) => "recipe",
            Self::NotInstalled(_) => "missing",
            Self::Failed(_) => "error",
        }
    }

    /// The program FFM would run now, when it has one.
    #[must_use]
    pub const fn resolved(&self) -> Option<&Resolved> {
        match self {
            Self::Configured(resolved)
            | Self::Cached(resolved)
            | Self::Replaced(Replacement::Cached(resolved)) => Some(resolved),
            Self::NotInstalled(_) | Self::Replaced(Replacement::AfterTrust) | Self::Failed(_) => {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
