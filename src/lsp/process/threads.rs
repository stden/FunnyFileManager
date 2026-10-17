//! Purpose: The bodies of a language server's I/O threads — framed reads,
//! framed writes, and a stderr drain that logs sizes, never content.

use std::io::{self, BufReader, Read};
use std::sync::mpsc::{Receiver, SyncSender};

use serde_json::Value;

use super::Signal;
use crate::lsp::transport;

pub(super) fn reader(input: impl Read, events: &SyncSender<Signal>) {
    let mut input = BufReader::new(input);
    loop {
        let event = match transport::read_frame(&mut input) {
            Ok(Some(message)) => Signal::Message(message),
            Ok(None) => Signal::Closed,
            Err(_) => Signal::Failed,
        };
        let finished = !matches!(event, Signal::Message(_));
        if events.send(event).is_err() || finished {
            return;
        }
    }
}

pub(super) fn writer(
    mut output: impl io::Write,
    messages: &Receiver<Value>,
    events: &SyncSender<Signal>,
) {
    for message in messages {
        if transport::write_frame(&mut output, &message).is_err() {
            // The receiver may already be gone during normal shutdown.
            drop(events.send(Signal::Failed));
            return;
        }
    }
}

pub(super) fn drain_stderr(mut stderr: impl Read) {
    let mut buffer = [0; 1024];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => return,
            Ok(bytes) => crate::diagnostics::log_error(&format!(
                "LSP stderr received ({bytes} bytes; content redacted)"
            )),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => {
                crate::diagnostics::log_error("LSP stderr read failed");
                return;
            }
        }
    }
}
