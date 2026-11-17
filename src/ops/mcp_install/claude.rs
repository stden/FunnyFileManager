//! Purpose: Claude Code's user-level registration, which goes through the
//! `claude` CLI: its user config also holds the sign-in session, so nothing
//! but Claude Code may write it.
//!
//! The CLI runs non-interactively, bounded by [`CLI_TIMEOUT`], with its output
//! captured. It is never run against a config that does not parse, and its
//! success is checked by reading the config again afterwards.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use super::report::{Failure, Outcome, Registration};
use super::survey;
use crate::mcp_clients::{
    self, Blocker, CliCall, ClientId, ConfigFormat, ConfigState, Host, ServerEntry,
};

mod cli;

use cli::{add_args, add_json_args, command_line, execute, remove_args};

/// How long one `claude` call may take. It normally needs about a second.
pub const CLI_TIMEOUT: Duration = Duration::from_secs(30);

/// What registering takes, from what the config holds now.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Plan {
    /// Nothing to run: the outcome is already known.
    Done(Outcome),
    Run(Steps),
}

/// The `claude` calls a registration makes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Steps {
    /// No entry: `claude mcp add`.
    Add,
    /// An entry that differs: `claude mcp remove`, since `add` refuses a name
    /// that exists, then `claude mcp add-json` with `entry` — the old entry
    /// with this server set, so its `env` and other keys survive, which
    /// `add` with only the command would drop.
    Replace { entry: String },
}

/// Register `desired` at user scope.
pub fn register(host: &Host, desired: &ServerEntry) -> Registration {
    let client = ClientId::ClaudeCode.client();
    let located = survey::client_home(host, client)
        .and_then(|home| survey::user_config(host, client).map(|config| (home, config)));
    let (home, config) = match located {
        Ok(located) => located,
        Err(blocker) => return Registration::stopped(client.id, None, Failure::Blocked(blocker)),
    };
    let (outcome, ran) = match plan(&config, desired) {
        Plan::Done(outcome) => (outcome, Vec::new()),
        Plan::Run(steps) => run(
            host,
            &Paths {
                home,
                config: &config,
            },
            desired,
            steps,
        ),
    };
    Registration {
        client: client.id,
        config: Some(config),
        ran,
        outcome,
    }
}

/// Where the CLI runs, and the config it is expected to change.
struct Paths<'a> {
    home: PathBuf,
    config: &'a Path,
}

fn plan(config: &Path, desired: &ServerEntry) -> Plan {
    let text = match survey::load(config) {
        Ok(text) => text.unwrap_or_default(),
        Err(blocker) => return blocked(blocker),
    };
    match mcp_clients::state(ConfigFormat::Json, &text, Some(desired)) {
        ConfigState::Registered | ConfigState::Disabled => Plan::Done(Outcome::Unchanged),
        ConfigState::Blocked(blocker) => blocked(blocker),
        ConfigState::NoConfig | ConfigState::NotConfigured => Plan::Run(Steps::Add),
        // With a server to compare against, an entry either matches or
        // differs; the other two states cannot come back here.
        ConfigState::DifferentEntry
        | ConfigState::Unverified { .. }
        | ConfigState::ProgramMissing => mcp_clients::claude_entry(&text, desired)
            .map_or_else(blocked, |entry| Plan::Run(Steps::Replace { entry })),
    }
}

/// A config that stops the registration before anything runs.
fn blocked(blocker: Blocker) -> Plan {
    Plan::Done(Outcome::Failed(Failure::Blocked(blocker)))
}

/// Run the CLI in the client's home folder and check what it wrote.
fn run(
    host: &Host,
    paths: &Paths<'_>,
    desired: &ServerEntry,
    steps: Steps,
) -> (Outcome, Vec<String>) {
    let Some(program) = survey::client_program(host, ClientId::ClaudeCode.client()) else {
        return (Outcome::Failed(Failure::CliMissing), Vec::new());
    };
    let call = |args: Vec<String>| CliCall {
        program: program.clone(),
        args,
        cwd: paths.home.clone(),
        timeout: CLI_TIMEOUT,
    };
    match steps {
        Steps::Add => add(host, paths.config, desired, &call(add_args(desired))),
        Steps::Replace { entry } => {
            let calls = [call(remove_args()), call(add_json_args(&entry))];
            replace(host, paths.config, desired, &calls)
        }
    }
}

fn add(host: &Host, config: &Path, desired: &ServerEntry, add: &CliCall) -> (Outcome, Vec<String>) {
    let by_hand = command_line(add);
    let outcome = match execute(host, add, &by_hand) {
        Err(failure) => Outcome::Failed(failure),
        Ok(()) if verified(config, desired) => Outcome::Added,
        Ok(()) => Outcome::Failed(Failure::NotVerified),
    };
    (outcome, vec![by_hand])
}

/// `remove`, then `add-json`. Until the remove has worked both are still to
/// run by hand; after it, the old entry is gone whatever the add does, and
/// the report has to say so.
fn replace(
    host: &Host,
    config: &Path,
    desired: &ServerEntry,
    [remove, add]: &[CliCall; 2],
) -> (Outcome, Vec<String>) {
    let (removing, adding) = (command_line(remove), command_line(add));
    if let Err(failure) = execute(host, remove, &format!("{removing} && {adding}")) {
        return (Outcome::Failed(failure), vec![removing]);
    }
    let ran = vec![removing, adding.clone()];
    let failure = match execute(host, add, &adding) {
        Ok(()) if verified(config, desired) => return (Outcome::Updated, ran),
        Ok(()) => Failure::NotVerified,
        Err(failure) => failure,
    };
    let removed = Failure::RemovedNotAdded {
        message: failure.reason(&config.display().to_string()),
        command: adding,
    };
    (Outcome::Failed(removed), ran)
}

/// Whether the config now registers `desired`.
fn verified(config: &Path, desired: &ServerEntry) -> bool {
    matches!(
        survey::read_state(config, ConfigFormat::Json, Some(desired)),
        ConfigState::Registered | ConfigState::Disabled
    )
}

#[cfg(test)]
mod tests;
