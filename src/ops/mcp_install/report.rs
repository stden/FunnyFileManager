//! Purpose: What registering did, client by client — the outcomes, the
//! "MCP server registration" report and the status line — kept apart from
//! the code that has the effects.

use std::path::PathBuf;

use super::claude::CLI_TIMEOUT;
use super::survey::Target;
use crate::mcp_clients::{Blocker, ClientId, ServerBinary};
use crate::status::Status;

/// How one client's registration ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Added,
    Updated,
    Unchanged,
    Failed(Failure),
}

impl Outcome {
    /// The report's word for it.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
            Self::Failed(_) => "failed",
        }
    }
}

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

/// One client's registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub client: ClientId,
    /// The config file, when its folder could be worked out.
    pub config: Option<PathBuf>,
    /// The commands run, as a person would type them.
    pub ran: Vec<String>,
    pub outcome: Outcome,
}

impl Registration {
    /// A registration that stopped before anything was run.
    #[must_use]
    pub const fn stopped(client: ClientId, config: Option<PathBuf>, failure: Failure) -> Self {
        Self {
            client,
            config,
            ran: Vec::new(),
            outcome: Outcome::Failed(failure),
        }
    }

    /// The report's last column: the failure, the commands run, or the file.
    fn detail(&self) -> String {
        let config = self.config.as_ref().map_or_else(
            || "the config".to_owned(),
            |path| path.display().to_string(),
        );
        match &self.outcome {
            Outcome::Failed(failure) => failure.describe(&config),
            _ if !self.ran.is_empty() => format!("ran: {}", self.ran.join("; ")),
            _ => config,
        }
    }
}

/// The title of the report viewer.
pub const REPORT_TITLE: &str = "MCP server registration";

/// Always the last note: nothing reads a config it has already loaded.
const RESTART_NOTE: &str =
    "Restart running clients, or reload their MCP servers, to pick up the change.";

/// The report: the server, the scope, one line per client, then the notes.
#[must_use]
pub fn lines(
    server: &ServerBinary,
    target: Target<'_>,
    registrations: &[Registration],
) -> Vec<String> {
    let scope = match target {
        Target::AllProjects => "all projects (user level)".to_owned(),
        Target::CurrentFolder(folder) => {
            format!("current folder (project level): {}", folder.as_str())
        }
    };
    let mut lines = vec![
        format!("ffm-mcp: {}", server.as_str()),
        format!("Scope: {scope}"),
        String::new(),
    ];
    lines.extend(registrations.iter().map(|registration| {
        format!(
            "{:<12} {:<10} {}",
            registration.client.name(),
            registration.outcome.word(),
            registration.detail()
        )
    }));
    lines.extend([String::new(), "Notes:".to_owned()]);
    lines.extend(
        registrations
            .iter()
            .filter_map(|registration| registration.client.client().note_line(target.scope()))
            .map(|note| format!("  {note}")),
    );
    lines.push(format!("  {RESTART_NOTE}"));
    lines
}

/// The status line: what changed, or which clients failed.
#[must_use]
pub fn status(registrations: &[Registration]) -> Status {
    let failed: Vec<&str> = registrations
        .iter()
        .filter(|registration| matches!(registration.outcome, Outcome::Failed(_)))
        .map(|registration| registration.client.name())
        .collect();
    if !failed.is_empty() {
        return Status::Error(format!(
            "ffm-mcp: {} failed; see the report",
            failed.join(", ")
        ));
    }
    let counts: Vec<String> = [Outcome::Added, Outcome::Updated, Outcome::Unchanged]
        .iter()
        .filter_map(|outcome| {
            let count = registrations
                .iter()
                .filter(|registration| registration.outcome == *outcome)
                .count();
            (count > 0).then(|| format!("{count} {}", outcome.word()))
        })
        .collect();
    Status::CommandOutput(format!("ffm-mcp: {}", counts.join(", ")))
}

#[cfg(test)]
mod tests;
