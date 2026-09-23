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
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

const TICK: Duration = Duration::from_millis(5);
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);

mod lifecycle;

struct Connection {
    owner: DocumentId,
    process: Process,
    client: Client,
    pending: Option<Snapshot>,
    query: Option<Query>,
    started: Instant,
}

pub(super) fn run(commands: &Receiver<Command>, events: &SyncSender<Envelope>, stop: &AtomicBool) {
    let mut host = lifecycle::Host::default();
    while !stop.load(Ordering::Acquire) {
        while let Ok(command) = commands.try_recv() {
            host.command(command, events);
        }
        host.pump(events);
        thread::sleep(TICK);
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
            owner: snapshot.stamp.id,
            process,
            client: Client::new(language, root_uri),
            pending: Some(snapshot),
            query: None,
            started: Instant::now(),
        })
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Update(snapshot) if snapshot.stamp.id == self.owner => {
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

    fn pump(&mut self, events: &SyncSender<Envelope>) -> Result<(), &'static str> {
        for _ in 0..32 {
            match self.process.poll() {
                Some(Signal::Message(message)) => self.client.receive(message)?,
                Some(Signal::Closed | Signal::Failed) => return Err("LSP disconnected"),
                None => break,
            }
        }
        if self.client.is_ready() {
            if let Some(snapshot) = self.pending.take() {
                self.client.synchronize(snapshot)?;
            }
            self.query(events);
        } else if self.started.elapsed() > INITIALIZE_TIMEOUT {
            return Err("LSP initialization timed out");
        }
        self.flush()?;
        for event in self.client.events.drain(..) {
            report(events, self.owner, event);
        }
        Ok(())
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

    fn flush(&mut self) -> Result<(), &'static str> {
        flush_messages(&mut self.client.rpc.outbox, |message| {
            self.process.send(message)
        })
    }

    fn shutdown(&mut self) {
        if !self.client.is_ready() {
            return;
        }
        self.client.shutdown();
        if self.flush().is_err() {
            return;
        }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        while Instant::now() < deadline && !self.client.shutdown_complete {
            if self
                .flush()
                .and_then(|()| self.shutdown_response())
                .is_err()
            {
                return;
            }
        }
        self.client.rpc.notify("exit", serde_json::Value::Null);
        if self.flush().is_err() {
            return;
        }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        while Instant::now() < deadline && !self.process.exited() {
            if self.flush().is_err() {
                return;
            }
            thread::sleep(TICK);
        }
    }

    fn shutdown_response(&mut self) -> Result<(), &'static str> {
        match self.process.poll() {
            Some(Signal::Message(message)) => self.client.receive(message)?,
            Some(Signal::Closed | Signal::Failed) => {
                return Err("LSP disconnected during shutdown")
            }
            None => thread::sleep(TICK),
        }
        Ok(())
    }
}

fn flush_messages(
    outbox: &mut Vec<serde_json::Value>,
    mut send: impl FnMut(serde_json::Value) -> Result<(), TrySendError<serde_json::Value>>,
) -> Result<(), &'static str> {
    if outbox.len() > 1024 {
        return Err("LSP writer backlog exceeded its limit");
    }
    let mut messages = std::mem::take(outbox).into_iter();
    while let Some(message) = messages.next() {
        match send(message) {
            Ok(()) => {}
            Err(TrySendError::Full(message)) => {
                outbox.push(message);
                outbox.extend(messages);
                break;
            }
            Err(TrySendError::Disconnected(_)) => return Err("LSP writer unavailable"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn report(events: &SyncSender<Envelope>, owner: DocumentId, event: Event) {
    // The UI receiver is intentionally dropped before the worker is joined.
    let _ = events.send(Envelope(owner, event));
}

fn workspace_root(path: &Path, markers: &[String]) -> PathBuf {
    let parent = path.parent().unwrap_or(path);
    parent
        .ancestors()
        .find(|directory| markers.iter().any(|marker| directory.join(marker).exists()))
        .unwrap_or(parent)
        .to_path_buf()
}
