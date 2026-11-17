//! Purpose: A connection's orderly end — `shutdown`, then `exit`, each waited
//! for no longer than its timeout — run whenever a connection is dropped.

use std::{sync::mpsc::TrySendError, thread, time::Instant};

use super::{Connection, SHUTDOWN_TIMEOUT, TICK};
use crate::lsp::process::Signal;

impl Connection {
    pub(super) fn flush(&mut self) -> Result<(), &'static str> {
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

    pub(super) fn close_document(&mut self) {
        self.owner = super::DocumentId::default();
        self.pending = None;
        self.query = None;
        self.client.close();
        let _ignored = self.flush();
    }

    pub(super) fn reopen(
        &mut self,
        snapshot: super::Snapshot,
        events: &super::SyncSender<super::Envelope>,
    ) {
        self.owner = snapshot.stamp.id;
        self.path.clone_from(&snapshot.path);
        self.pending = Some(snapshot);
        self.query = None;
        let state = if self.client.is_ready() {
            super::ServerState::Ready
        } else {
            super::ServerState::Initializing
        };
        super::report(events, self.owner, super::Event::State(state));
    }
}

pub(super) fn flush_messages(
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

impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown();
    }
}
