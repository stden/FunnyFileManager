//! Purpose: Serialize server lifecycle and documents away from the interactive thread.

use super::{
    client::{Client, Event},
    manager::{Command, Envelope},
    process::{Process, Signal},
    types::{DocumentId, Query, ServerState, Snapshot},
    uri,
};
use crate::config::ServerSettings;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender},
    },
    time::{Duration, Instant},
};

const TICK: Duration = Duration::from_millis(5);
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);

mod lifecycle;
mod shutdown;
#[cfg(test)]
use shutdown::flush_messages;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PumpError {
    Disconnected(&'static str),
    MissingComponent(&'static str),
}

struct Connection {
    language: String,
    root: PathBuf,
    configured: ServerSettings,
    owner: DocumentId,
    path: PathBuf,
    process: Process,
    client: Client,
    pending: Option<Snapshot>,
    query: Option<Query>,
    started: Instant,
}

pub(super) fn run(commands: &Receiver<Command>, events: &SyncSender<Envelope>, stop: &AtomicBool) {
    let mut host = lifecycle::Host::default();
    host.cache = super::install::cache_dir();
    while !stop.load(Ordering::Acquire) {
        if !wait_or_dispatch(&mut host, commands, events) {
            break;
        }
        while let Ok(command) = commands.try_recv() {
            host.command(command, events);
        }
        host.pump(events);
    }
}

fn wait_or_dispatch(
    host: &mut lifecycle::Host,
    commands: &Receiver<Command>,
    events: &SyncSender<Envelope>,
) -> bool {
    if host.is_active() {
        match commands.recv_timeout(TICK) {
            Ok(command) => host.command(command, events),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return false,
        }
        true
    } else {
        commands.recv().is_ok_and(|command| {
            host.command(command, events);
            true
        })
    }
}

impl Connection {
    fn start(
        language: String,
        settings: &ServerSettings,
        snapshot: Snapshot,
        root: &Path,
    ) -> std::io::Result<Self> {
        let root_uri = uri::from_path(root)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let process = Process::start(settings, root)?;
        Ok(Self {
            language: language.clone(),
            root: root.to_path_buf(),
            configured: settings.clone(),
            owner: snapshot.stamp.id,
            path: snapshot.path.clone(),
            process,
            client: Client::new(language, root_uri),
            pending: Some(snapshot),
            query: None,
            started: Instant::now(),
        })
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Update(mut snapshot) if snapshot.stamp.id == self.owner => {
                snapshot.path.clone_from(&self.path);
                self.pending = Some(snapshot);
                self.query = None;
            }
            Command::Query(query) if query.stamp.id == self.owner => self.query = Some(query),
            Command::Cancel => {
                self.query = None;
                self.client.rpc.cancel_queries();
            }
            // Open/close are handled by the manager; obsolete documents are ignored.
            _ => {}
        }
    }

    fn pump(&mut self, events: &SyncSender<Envelope>) -> Result<(), PumpError> {
        for _ in 0..32 {
            match self.process.poll() {
                Some(Signal::Message(message)) => {
                    self.client
                        .receive(message)
                        .map_err(PumpError::Disconnected)?;
                }
                Some(Signal::Closed | Signal::Failed) => return Err(self.closed_error()),
                None => break,
            }
        }
        if self.client.is_ready() {
            if let Some(snapshot) = self.pending.take() {
                self.client
                    .synchronize(snapshot)
                    .map_err(PumpError::Disconnected)?;
            }
            self.query(events);
        } else if self.started.elapsed() > INITIALIZE_TIMEOUT {
            return Err(PumpError::Disconnected("LSP initialization timed out"));
        }
        self.flush().map_err(PumpError::Disconnected)?;
        for event in self.client.events.drain(..) {
            report(events, self.owner, event);
        }
        Ok(())
    }

    fn closed_error(&mut self) -> PumpError {
        if !self.client.is_ready() {
            if let Some(hint) = self.process.take_toolchain_hint() {
                return PumpError::MissingComponent(hint);
            }
        }
        PumpError::Disconnected("LSP disconnected")
    }

    fn query(&mut self, events: &SyncSender<Envelope>) {
        if let Some(query) = self.query.take() {
            if let Err(message) = self.client.query(query) {
                let error = super::rpc::ResponseError {
                    code: -32601,
                    message: message.into(),
                    data: serde_json::Value::Null,
                };
                report(events, self.owner, Event::Reply(query, Err(error)));
            }
        }
    }
}

#[cfg(test)]
mod tests;

fn report(events: &SyncSender<Envelope>, owner: DocumentId, event: Event) {
    // The UI receiver is intentionally dropped before the worker is joined.
    drop(events.send(Envelope(owner, event)));
}

pub(super) fn workspace_root(path: &Path, markers: &[String]) -> PathBuf {
    let start = path.parent().unwrap_or(path);
    let home = crate::config::user_home();
    start
        .ancestors()
        .take_while(|directory| {
            *directory == start
                || (directory.parent().is_some() && home.as_deref().is_none_or(|h| h != *directory))
        })
        .find(|directory| markers.iter().any(|marker| directory.join(marker).exists()))
        .unwrap_or(start)
        .to_path_buf()
}
