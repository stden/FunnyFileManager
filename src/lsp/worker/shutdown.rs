//! Purpose: A connection's orderly end — `shutdown`, then `exit`, each waited
//! for no longer than its timeout — run whenever a connection is dropped.

use std::{thread, time::Instant};

use super::{Connection, SHUTDOWN_TIMEOUT, TICK};
use crate::lsp::process::Signal;

impl Connection {
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

impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown();
    }
}
