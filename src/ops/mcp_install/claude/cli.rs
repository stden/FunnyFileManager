//! Purpose: The `claude mcp` calls a registration makes, the line a person
//! would type for each, and running one through the host's runner.

use std::io;

use super::super::report::Failure;
use crate::mcp_clients::{CliCall, Exit, Host, ServerEntry, SERVER_NAME};

pub(super) fn add_args(desired: &ServerEntry) -> Vec<String> {
    words(&[
        "mcp",
        "add",
        "--scope",
        "user",
        "--transport",
        "stdio",
        SERVER_NAME,
        "--",
    ])
    .chain([desired.command.as_str().to_owned()])
    .collect()
}

pub(super) fn add_json_args(entry: &str) -> Vec<String> {
    words(&["mcp", "add-json", "--scope", "user", SERVER_NAME])
        .chain([entry.to_owned()])
        .collect()
}

pub(super) fn remove_args() -> Vec<String> {
    words(&["mcp", "remove", "--scope", "user", SERVER_NAME]).collect()
}

fn words<'a>(list: &'a [&'a str]) -> impl Iterator<Item = String> + 'a {
    list.iter().map(|word| (*word).to_owned())
}

/// The call as a person would type it into the platform's shell.
pub(super) fn command_line(call: &CliCall) -> String {
    std::iter::once("claude".to_owned())
        .chain(
            call.args
                .iter()
                .map(|word| crate::console::shell_word(word)),
        )
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run one call; anything but a clean exit is a [`Failure`].
pub(super) fn execute(host: &Host, call: &CliCall, by_hand: &str) -> Result<(), Failure> {
    let captured = host
        .runner
        .run(call)
        .map_err(|error| unfinished(call, &error, by_hand))?;
    match captured.exit {
        Exit::Success => Ok(()),
        Exit::Failure(exit) => Err(Failure::CliFailed {
            exit,
            // The last line is where a CLI says what went wrong.
            message: crate::console::first_drawable(captured.stderr.lines().rev())
                .or_else(|| crate::console::first_drawable(captured.stdout.lines().rev()))
                .unwrap_or_else(|| "claude failed without saying why".to_owned()),
            command: by_hand.to_owned(),
        }),
    }
}

/// Why a run did not finish. `NotFound` means a missing `claude` only when
/// the program is gone: a working folder or a temp folder that is missing
/// says `NotFound` too, and that is not the CLI's absence.
fn unfinished(call: &CliCall, error: &io::Error, by_hand: &str) -> Failure {
    match error.kind() {
        io::ErrorKind::NotFound if !call.program.is_file() => Failure::CliMissing,
        io::ErrorKind::TimedOut => Failure::CliTimedOut {
            command: by_hand.to_owned(),
        },
        _ => Failure::CliFailed {
            exit: None,
            message: error.to_string(),
            command: by_hand.to_owned(),
        },
    }
}
