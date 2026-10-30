//! Purpose: The bodies of a language server's I/O threads — framed reads,
//! framed writes, and a stderr drain that logs sizes, never content.

use std::io::{self, BufReader, Read};
use std::sync::mpsc::{Receiver, SyncSender};

use serde_json::Value;

use super::Signal;
use crate::lsp::transport;

const STDERR_SNIFF_LIMIT: usize = 512;
const RUSTUP_RUST_ANALYZER_HINT: &str =
    "LSP unavailable: run 'rustup component add rust-analyzer' (Ctrl+Alt+R retries)";
const RUSTUP_COMPONENT_HINT: &str =
    "LSP unavailable: missing rustup toolchain component (Ctrl+Alt+R retries)";

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

#[derive(Default)]
struct StderrCapture {
    prefix: Vec<u8>,
    deferred_bytes: usize,
    streamed: bool,
}

impl StderrCapture {
    fn push(&mut self, chunk: &[u8]) {
        if self.prefix.len() < STDERR_SNIFF_LIMIT {
            let room = STDERR_SNIFF_LIMIT - self.prefix.len();
            let count = room.min(chunk.len());
            self.prefix
                .extend_from_slice(chunk.get(..count).unwrap_or(&[]));
        }
        if !self.streamed && self.deferred_bytes + chunk.len() <= STDERR_SNIFF_LIMIT {
            self.deferred_bytes += chunk.len();
            return;
        }
        if !self.streamed && self.deferred_bytes > 0 {
            log_redacted(self.deferred_bytes);
            self.deferred_bytes = 0;
        }
        self.streamed = true;
        log_redacted(chunk.len());
    }

    fn finish(self, failed: bool) -> Option<&'static str> {
        if failed {
            if self.deferred_bytes > 0 {
                log_redacted(self.deferred_bytes);
            }
            crate::diagnostics::log_error("LSP stderr read failed");
            return None;
        }
        if !self.streamed {
            if let Some(hint) = toolchain_hint(&self.prefix) {
                return Some(hint);
            }
            if self.deferred_bytes > 0 {
                log_redacted(self.deferred_bytes);
            }
        }
        None
    }
}

fn log_redacted(bytes: usize) {
    crate::diagnostics::log_error(&format!(
        "LSP stderr received ({bytes} bytes; content redacted)"
    ));
}

fn toolchain_hint(bytes: &[u8]) -> Option<&'static str> {
    let text = String::from_utf8_lossy(bytes);
    let missing = text.contains("is not installed for the toolchain")
        || (text.contains("Unknown binary '") && text.contains("in official toolchain"));
    if !missing {
        return None;
    }
    if text.contains("rust-analyzer") {
        Some(RUSTUP_RUST_ANALYZER_HINT)
    } else {
        Some(RUSTUP_COMPONENT_HINT)
    }
}

pub(super) fn drain_stderr(mut stderr: impl Read) -> Option<&'static str> {
    let mut buffer = [0; 1024];
    let mut capture = StderrCapture::default();
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) => return capture.finish(false),
            Ok(bytes) => capture.push(buffer.get(..bytes).unwrap_or(&[])),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return capture.finish(true),
        }
    }
}
