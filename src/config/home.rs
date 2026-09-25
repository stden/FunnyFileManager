//! Purpose: Where the user's files live — the home folder, the `ffm`
//! configuration directory and the `ffm` data directory — as one rule every
//! caller shares.
//!
//! Each variable counts only when it holds an absolute path. The XDG spec
//! says so of its base directories, and an empty or relative `HOME` is no
//! home either: under `HOME=` the settings, the histories and a `.profile`
//! landed in the launch directory, a different set for every folder FFM was
//! started in. Without a home there is no configuration and no data
//! directory, and the session keeps its settings in memory.

use std::{ffi::OsString, path::PathBuf};

/// The user's home folder; `None` when the environment names none.
#[must_use]
pub fn user_home() -> Option<PathBuf> {
    Platform::CURRENT.home(std::env::var_os::<&str>)
}

/// The `ffm` directory every configuration file lives in — the settings, the
/// user menu and the file associations — so that they can be backed up as
/// one. `None` when the environment names no home at all.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    Platform::CURRENT.dir(std::env::var_os::<&str>)
}

/// The `ffm` directory the histories, the bookmarks and other data live in.
/// `None` when the environment names no home at all.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    Platform::CURRENT.data_dir(std::env::var_os::<&str>)
}

/// Whose rule names the directories. A value rather than two `#[cfg]`
/// functions, so that both rules run in every build's tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Platform {
    /// `$HOME` when set (Git Bash, MSYS2 and dotfile setups set it on
    /// purpose), else `%USERPROFILE%`; `%APPDATA%` for configuration.
    Windows,
    /// `$HOME`, and `$XDG_CONFIG_HOME` (else `~/.config`) for configuration.
    Unix,
}

impl Platform {
    /// The platform this build runs on.
    pub(super) const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Unix
    };

    /// The home folder, reading the environment through `variable`.
    pub(super) fn home(
        self,
        variable: impl Fn(&'static str) -> Option<OsString>,
    ) -> Option<PathBuf> {
        let names: &[&'static str] = match self {
            Self::Windows => &["HOME", "USERPROFILE"],
            Self::Unix => &["HOME"],
        };
        names.iter().find_map(|name| absolute(variable(name)))
    }

    /// The `ffm` directory under this platform's configuration home.
    pub(super) fn dir(
        self,
        variable: impl Fn(&'static str) -> Option<OsString>,
    ) -> Option<PathBuf> {
        let base = match self {
            Self::Windows => absolute(variable("APPDATA")),
            Self::Unix => absolute(variable("XDG_CONFIG_HOME"))
                .or_else(|| Some(self.home(&variable)?.join(".config"))),
        };
        base.map(|base| base.join("ffm"))
    }

    /// The `ffm` directory under `$XDG_DATA_HOME`, else `~/.local/share` —
    /// on Windows too, where the Colorer schemes are installed there.
    pub(super) fn data_dir(
        self,
        variable: impl Fn(&'static str) -> Option<OsString>,
    ) -> Option<PathBuf> {
        absolute(variable("XDG_DATA_HOME"))
            .or_else(|| Some(self.home(&variable)?.join(".local").join("share")))
            .map(|base| base.join("ffm"))
    }
}

/// `value` as a path, when it is an absolute one.
fn absolute(value: Option<OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|path| path.is_absolute())
}
