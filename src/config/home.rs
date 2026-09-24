//! Purpose: Where the configuration lives — the `ffm` directory under each
//! platform's configuration home.

use std::{ffi::OsString, path::PathBuf};

/// The `ffm` directory every configuration file lives in — the settings, the
/// user menu and the file associations — so that they can be backed up as
/// one. `None` when the environment names no home at all.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    Platform::CURRENT.dir(std::env::var_os::<&str>)
}

/// Whose rule names the configuration home. A value rather than two
/// `#[cfg]` functions, so that both rules run in every build's tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Platform {
    /// `%APPDATA%`.
    Windows,
    /// `$XDG_CONFIG_HOME` when it is absolute, else `$HOME/.config`.
    Unix,
}

impl Platform {
    /// The platform this build runs on.
    pub(super) const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Unix
    };

    /// The `ffm` directory under this platform's configuration home, reading
    /// the environment through `variable`. A relative `$XDG_CONFIG_HOME` is
    /// ignored, as the XDG spec says, so settings never land in the launch
    /// directory.
    pub(super) fn dir(
        self,
        variable: impl Fn(&'static str) -> Option<OsString>,
    ) -> Option<PathBuf> {
        let home = match self {
            Self::Windows => variable("APPDATA").map(PathBuf::from),
            Self::Unix => variable("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .or_else(|| variable("HOME").map(|home| PathBuf::from(home).join(".config"))),
        };
        home.map(|home| home.join("ffm"))
    }
}
