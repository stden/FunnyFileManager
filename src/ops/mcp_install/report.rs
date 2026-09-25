//! Purpose: What registering did, client by client — the outcomes, the
//! "MCP server registration" report and the status line — kept apart from
//! the code that has the effects.

use std::path::PathBuf;

use super::survey::Target;
use crate::mcp_clients::{Blocker, ClientId, ClientStatus, ConfigState, ServerBinary};
use crate::status::Status;

mod failure;

pub use failure::Failure;

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

/// The heading of the configs that cannot be read or do not parse.
pub const CONFIG_PROBLEMS: &str = "Configs FFM cannot use, left untouched:";

/// Always the last note: nothing reads a config it has already loaded.
const RESTART_NOTE: &str =
    "Restart running clients, or reload their MCP servers, to pick up the change.";

/// The report: the server, the scope, one line per client, the configs
/// that could not be used (a [`problem_section`]), then the notes.
#[must_use]
pub fn lines(
    server: &ServerBinary,
    target: Target<'_>,
    registrations: &[Registration],
    problems: &[String],
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
    lines.extend_from_slice(problems);
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

/// Every config in `statuses` that cannot be read or does not parse, one
/// line each under [`CONFIG_PROBLEMS`] with why — the parser's line and
/// column where it gives them; nothing when every config is usable. The
/// dialog only dims such a row, so this is where the reason is read.
#[must_use]
pub fn problem_section<'a>(statuses: impl IntoIterator<Item = &'a ClientStatus>) -> Vec<String> {
    let problems: Vec<String> = statuses.into_iter().filter_map(config_problem).collect();
    if problems.is_empty() {
        return problems;
    }
    [String::new(), CONFIG_PROBLEMS.to_owned()]
        .into_iter()
        .chain(problems)
        .collect()
}

/// `status`'s line in a [`problem_section`], when its config cannot be used.
fn config_problem(status: &ClientStatus) -> Option<String> {
    let ConfigState::Blocked(
        blocker @ (Blocker::Unparsable(_) | Blocker::Unreadable(_) | Blocker::UnexpectedLayout(_)),
    ) = &status.state
    else {
        return None;
    };
    let config = status.config.as_ref()?.display().to_string();
    Some(format!(
        "  {:<12} {}",
        status.client.name(),
        blocker.explain(&config)
    ))
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
