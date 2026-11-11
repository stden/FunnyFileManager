//! Purpose: Surface canonical transfer and directory-read progress as structured state without terminal output.
use super::published::Shared;
use crate::progress::{DirectoryRead, Painter, Snapshot};
use serde_json::{json, Value};

pub(super) struct Reporter(pub(super) Shared);

impl Reporter {
    fn publish(&self, payload: Value) {
        if let Err(error) = self.0.progress(payload) {
            crate::diagnostics::log_error(&error.to_string());
        }
    }
}

impl Painter for Reporter {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.publish(describe(snapshot));
    }

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        self.publish(json!({
            "operation": "DirectoryRead",
            "files": read.files,
        }));
    }
}

fn describe(snapshot: &Snapshot) -> Value {
    let transfer = snapshot.transfer.as_ref().map(|transfer| {
        json!({
            "destination": transfer.destination.to_string_lossy(), "file": transfer.file,
            "permille": transfer.file.permille(),
        })
    });
    json!({"operation": snapshot.operation, "phase": snapshot.phase,
        "current": snapshot.current.to_string_lossy(), "transfer": transfer,
        "done": snapshot.done, "total": snapshot.total, "permille": snapshot.permille(),
        "bytes_per_second": snapshot.bytes_per_second(),
        "elapsed_ms": u64::try_from(snapshot.elapsed.as_millis()).unwrap_or(u64::MAX)})
}

#[cfg(test)]
mod tests;
