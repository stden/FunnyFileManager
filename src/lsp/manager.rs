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

pub enum Command {
    Open(String, ServerSettings, Snapshot),
    Update(Snapshot),
    Query(Query),
    Cancel,
    Close,
}

pub struct Envelope(pub DocumentId, pub Event);

pub struct Manager {
    commands: SyncSender<Command>,
    events: Option<Receiver<Envelope>>,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
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
            commands,
            events: Some(events),
            stop,
            task: Some(task),
        })
    }

    pub fn send(&self, command: Command) -> Result<(), Box<Command>> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(command) | mpsc::TrySendError::Disconnected(command) => {
                    Box::new(command)
                }
            })
    }

    pub fn poll(&self) -> Option<Envelope> {
        self.events.as_ref()?.try_recv().ok()
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.events.take(); // Release a worker sending to a full UI mailbox before joining.
        if self.task.take().is_some_and(|task| task.join().is_err()) {
            crate::diagnostics::log_error("LSP worker panicked");
        }
    }
}

#[cfg(test)]
mod tests;
