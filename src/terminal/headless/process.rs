//! Purpose: Own a private pseudoterminal and reap its external child.

use super::{
    child::{Descendants, OwnedChild},
    master::Master,
    state::Generation,
    streams, ExternalOutcome, ExternalState, HeadlessHost,
};
use portable_pty::{native_pty_system, CommandBuilder};
use std::{
    io,
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub(super) fn run(
    host: &HeadlessHost,
    command: &Command,
    timeout: Option<Duration>,
    descendants: Descendants,
) -> io::Result<ExternalOutcome> {
    let (input, receiver) = mpsc::sync_channel(8);
    let generation = host.begin(input)?;
    let result = execute(host, generation, command, receiver, timeout, descendants);
    let state = match &result {
        Ok(outcome) => ExternalState::Exited { code: outcome.code },
        Err(error) if error.kind() == io::ErrorKind::Interrupted => ExternalState::Cancelled,
        Err(_) => ExternalState::Failed,
    };
    host.finish(generation, state);
    result
}

fn execute(
    host: &HeadlessHost,
    generation: Generation,
    command: &Command,
    input: mpsc::Receiver<Vec<u8>>,
    timeout: Option<Duration>,
    descendants: Descendants,
) -> io::Result<ExternalOutcome> {
    let pair = native_pty_system()
        .openpty(host.pty_size())
        .map_err(pty_error)?;
    let reader = pair.master.try_clone_reader().map_err(pty_error)?;
    let writer = pair.master.take_writer().map_err(pty_error)?;
    let mut child = OwnedChild::new(
        pair.slave
            .spawn_command(builder(command))
            .map_err(pty_error)?,
        pair.master,
        descendants,
    )?;
    drop(pair.slave);
    let (replies, responses) = mpsc::sync_channel(8);
    let output = streams::read(reader, host.clone(), generation, replies, child.master())?;
    streams::write(writer, input, responses, host.clone(), generation)?;
    let status = wait(&mut child, host, timeout);
    if status.is_err() {
        child.stop()?;
    }
    drop(child);
    let bytes = streams::collect(output)?;
    Ok(ExternalOutcome {
        code: status?.exit_code(),
        output: bytes,
    })
}

fn builder(command: &Command) -> CommandBuilder {
    let mut builder = CommandBuilder::new(command.get_program());
    builder.args(command.get_args());
    if let Some(directory) = command.get_current_dir() {
        #[cfg(windows)]
        let directory = crate::windows::console_directory(directory);
        builder.cwd(directory);
    }
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => builder.env(name, value),
            None => builder.env_remove(name),
        }
    }
    builder.env("TERM", "xterm-256color");
    builder
}

fn wait(
    child: &mut OwnedChild,
    host: &HeadlessHost,
    timeout: Option<Duration>,
) -> io::Result<portable_pty::ExitStatus> {
    let deadline = timeout.map(|limit| Instant::now() + limit);
    let mut size = child.master().size()?;
    loop {
        resize(child.master(), host, &mut size)?;
        if let Some(status) = child.poll()? {
            return Ok(status);
        }
        if host.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Command cancelled",
            ));
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "Command timed out"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn pty_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("Cannot create private terminal: {error}"))
}

fn resize(
    master: &Master,
    host: &HeadlessHost,
    previous: &mut portable_pty::PtySize,
) -> io::Result<()> {
    let size = host.pty_size();
    if size != *previous {
        master.resize(size)?;
        *previous = size;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
