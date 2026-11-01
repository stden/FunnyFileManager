//! Purpose: Surface canonical transfer progress as structured state without terminal output.
use super::published::Shared;
use crate::progress::{Painter, Snapshot};
use serde_json::{json, Value};

pub(super) struct Reporter(pub(super) Shared);

impl Painter for Reporter {
    fn paint(&mut self, snapshot: &Snapshot) {
        if let Err(error) = self.0.progress(describe(snapshot)) {
            crate::diagnostics::log_error(&error.to_string());
        }
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
