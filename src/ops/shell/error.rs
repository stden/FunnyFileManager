//! Purpose: The ways a shell run can end without output to report, and the one
//! sentence each reads as in the transcript and the status bar.
//!
//! A separate file because two operations report these (`:` commands and the
//! Apply command) and they must word them the same way.

use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fmt, io};

/// Why a shell command has no output to report.
#[derive(Debug)]
pub(crate) enum ShellError {
    /// The shell could not be started, or waiting for it failed.
    Launch(io::Error),
    /// The folder it was to run in is gone, typically deleted after the panel
    /// listed it. Told apart from `Launch` because the OS reports it as "No
    /// such file or directory", which reads as if the command were missing.
    MissingFolder(PathBuf),
    /// It ran for the whole time limit and was stopped.
    TimedOut(Duration),
    /// The user pressed Esc or Ctrl+C, and it was stopped.
    Cancelled,
}

impl ShellError {
    /// `error` from starting a process in `folder`, naming the folder when
    /// it is the folder that is missing.
    ///
    /// Asked after the fact rather than checked up front: only a failed start
    /// needs the answer, and a folder that cannot be looked up (no permission)
    /// is not reported as gone.
    pub(crate) fn launching_in(folder: &Path, error: io::Error) -> Self {
        if matches!(folder.try_exists(), Ok(false)) {
            return Self::MissingFolder(folder.to_path_buf());
        }
        Self::Launch(error)
    }

    /// What the transcript and the status bar say about `command` ending so.
    ///
    /// Only a launch failure says "cannot run": a command that timed out or
    /// was cancelled did run, and saying otherwise sends the reader looking
    /// for a missing program.
    pub(crate) fn report(&self, command: &str) -> String {
        match self {
            Self::Launch(_) | Self::MissingFolder(_) => format!("cannot run '{command}': {self}"),
            Self::TimedOut(_) => format!("stopped '{command}': {self}"),
            Self::Cancelled => format!("{self}: {command}"),
        }
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Launch(error) => write!(formatter, "{error}"),
            Self::MissingFolder(folder) => {
                write!(formatter, "folder {} no longer exists", folder.display())
            }
            Self::TimedOut(limit) => write!(
                formatter,
                "timed out after {}s (Esc stops it sooner)",
                limit.as_secs()
            ),
            Self::Cancelled => formatter.write_str("cancelled"),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Launch(error) => Some(error),
            Self::MissingFolder(_) | Self::TimedOut(_) | Self::Cancelled => None,
        }
    }
}
