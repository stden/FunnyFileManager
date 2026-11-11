//! Purpose: Why a registration did not happen, and how the report puts it:
//! the cause alone, or the cause with what to run by hand.

use std::path::PathBuf;

use super::super::claude::CLI_TIMEOUT;
use crate::mcp_clients::Blocker;

/// Why a registration did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The config was left alone, and why.
    Blocked(Blocker),
    /// Writing the file failed; the old file is as it was.
    Io { path: PathBuf, message: String },
    /// `claude` was gone by the time it was run.
    CliMissing,
    /// `claude` ran past [`CLI_TIMEOUT`] and was stopped. `command` is what
    /// to run by hand.
    CliTimedOut { command: String },
    CliFailed {
        exit: Option<i32>,
        message: String,
        command: String,
    },
    /// `claude mcp remove` worked and `claude mcp add` did not: the old entry
    /// is gone.
    RemovedNotAdded { message: String, command: String },
    /// `claude mcp add` reported success, but the config says otherwise.
    NotVerified,
}

impl Failure {
    /// The cause, without advice. `config` names the file when it is known.
    #[must_use]
    pub fn reason(&self, config: &str) -> String {
        match self {
            Self::Blocked(blocker) => blocker.explain(config),
            Self::Io { path, message } => format!("cannot write {}: {message}", path.display()),
            Self::CliMissing => "claude CLI not found".to_owned(),
            Self::CliTimedOut { .. } => {
                format!("claude did not finish within {} s", CLI_TIMEOUT.as_secs())
            }
            Self::CliFailed {
                exit: Some(code),
                message,
                ..
            } => format!("{message} (exit {code})"),
            Self::CliFailed {
                exit: None,
                message,
                ..
            }
            | Self::RemovedNotAdded { message, .. } => message.clone(),
            Self::NotVerified => {
                format!("claude reported success but {config} has no matching ffm-mcp entry")
            }
        }
    }

    /// The cause, and what to do about it.
    #[must_use]
    pub fn describe(&self, config: &str) -> String {
        match self {
            Self::CliTimedOut { command } | Self::CliFailed { command, .. } => {
                format!("{}. Run by hand: {command}", self.reason(config))
            }
            Self::RemovedNotAdded { message, command } => format!(
                "the old entry was removed but the new one was not added: {message}. \
                 Run by hand: {command}"
            ),
            Self::Blocked(_) | Self::Io { .. } | Self::CliMissing | Self::NotVerified => {
                self.reason(config)
            }
        }
    }
}
