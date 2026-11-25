//! Purpose: Bounded nonblocking UI mailbox and lifetime of the language-server worker.

pub use super::client::Event;
use super::types::{DocumentId, Query, Snapshot};
use crate::config::ServerSettings;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};
use std::thread::{self, JoinHandle};

#[derive(Debug)]
pub enum Command {
    Open(String, ServerSettings, Snapshot),
    Restart,
    Trust(DocumentId),
    Update(Snapshot),
    Query(Query),
    Cancel,
    Close,
}

pub struct Envelope(pub DocumentId, pub Event);

pub struct Manager {
    commands: Option<SyncSender<Command>>,
    events: Option<Receiver<Envelope>>,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub enum SendError {
    Full(Box<Command>),
    Disconnected(Box<Command>),
}

impl std::ops::Deref for SendError {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Full(command) | Self::Disconnected(command) => command,
        }
    }
}

impl Manager {
    pub fn start() -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let (sender, events) = mpsc::sync_channel(32);
        let stop = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::clone(&stop);
        let task = thread::Builder::new()
            .name("ffm-lsp".into())
            .spawn(move || super::worker::run(&receiver, &sender, &shutdown))?;
        Ok(Self {
            commands: Some(commands),
            events: Some(events),
            stop,
            task: Some(task),
        })
    }

    pub fn send(&self, command: Command) -> Result<(), SendError> {
        let Some(commands) = &self.commands else {
            return Err(SendError::Disconnected(Box::new(command)));
        };
        commands.try_send(command).map_err(|error| match error {
            mpsc::TrySendError::Full(command) => SendError::Full(Box::new(command)),
            mpsc::TrySendError::Disconnected(command) => SendError::Disconnected(Box::new(command)),
        })
    }

    pub fn poll(&self) -> Option<Envelope> {
        self.events.as_ref()?.try_recv().ok()
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.commands.take(); // Wake an idle worker blocked on `commands.recv()`.
        self.events.take(); // Release a worker sending to a full UI mailbox before joining.
        if self.task.take().is_some_and(|task| task.join().is_err()) {
            crate::diagnostics::log_error("LSP worker panicked");
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub mod tests_fixture;
