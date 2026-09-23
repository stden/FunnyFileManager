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
        mpsc::{Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

const TICK: Duration = Duration::from_millis(5);
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(750);

struct Connection {
    owner: DocumentId,
    process: Process,
    client: Client,
    pending: Option<Snapshot>,
    query: Option<Query>,
    started: Instant,
}

pub(super) fn run(commands: &Receiver<Command>, events: &SyncSender<Envelope>, stop: &AtomicBool) {
    let mut connection = None;
    while !stop.load(Ordering::Acquire) {
        while let Ok(command) = commands.try_recv() {
            dispatch(&mut connection, command, events);
        }
        if let Some(active) = &mut connection {
            if let Err(message) = active.pump(events) {
                crate::diagnostics::log_error(message);
                report(events, active.owner, Event::State(ServerState::Failed));
                connection.take();
            }
        }
        thread::sleep(TICK);
    }
}

fn dispatch(connection: &mut Option<Connection>, command: Command, events: &SyncSender<Envelope>) {
    match command {
        Command::Open(language, settings, snapshot) => {
            connection.take();
            let owner = snapshot.stamp.id;
            report(events, owner, Event::State(ServerState::Starting));
            match Connection::start(language, &settings, snapshot) {
                Ok(active) => {
                    *connection = Some(active);
                    report(events, owner, Event::State(ServerState::Initializing));
                }
                Err(error) => {
                    crate::diagnostics::log_error(&format!("LSP start failed: {:?}", error.kind()));
                    let state = if error.kind() == std::io::ErrorKind::NotFound {
                        ServerState::Unavailable
                    } else {
                        ServerState::Failed
                    };
                    report(events, owner, Event::State(state));
                }
            }
        }
        Command::Close => {
            connection.take();
        }
        command => {
            if let Some(active) = connection {
                active.command(command);
            }
        }
    }
}

impl Connection {
    fn start(
        language: String,
        settings: &ServerSettings,
        mut snapshot: Snapshot,
    ) -> std::io::Result<Self> {
        snapshot.path = std::path::absolute(&snapshot.path)?;
        let root = workspace_root(&snapshot.path, &settings.root_markers);
        let root_uri = uri::from_path(&root)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        let process = Process::start(settings, &root)?;
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
        for message in self.client.rpc.outbox.drain(..) {
            self.process
                .send(message)
                .map_err(|_| "LSP writer unavailable")?;
        }
        Ok(())
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
            if self.shutdown_response().is_err() {
                return;
            }
        }
        self.client.rpc.notify("exit", serde_json::Value::Null);
        if self.flush().is_err() {
            return;
        }
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        while Instant::now() < deadline && !self.process.exited() {
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
