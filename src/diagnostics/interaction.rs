//! Purpose: Correlate user input, its outcome and latency without recording typed payloads.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::models::App;

pub struct Interaction {
    id: u64,
    action: String,
    started: Instant,
}

impl Interaction {
    pub fn start(action: String, app: &App) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let interaction = Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            action,
            started: Instant::now(),
        };
        super::log_action(&format!(
            "INPUT id={} action={} panel={:?} view={:?} cursor={:?}",
            interaction.id,
            interaction.action,
            app.active_panel,
            app.view,
            app.get_active_panel_ref().selected()
        ));
        interaction
    }

    pub fn finish<T>(self, app: &App, result: &std::io::Result<T>) {
        let outcome = if result.is_err() { "error" } else { "handled" };
        super::log_action(&completion(
            self.id,
            &self.action,
            self.started.elapsed(),
            outcome,
        ));
        if let Err(error) = result {
            super::log_error(&format!(
                "INPUT id={} error_kind={:?}",
                self.id,
                error.kind()
            ));
        }
        // Variant names identify operation outcomes without logging command output or secrets.
        super::log_action(&format!(
            "OUTCOME id={} status={}",
            self.id,
            status_kind(&app.status)
        ));
    }
}

fn status_kind(status: &crate::status::Status) -> &'static str {
    use crate::status::Status;
    match status {
        Status::Error(_) => "Error",
        Status::CannotRun { .. } => "CannotRun",
        Status::ProgramExited { .. } => "ProgramExited",
        Status::CommandOutput(_) => "CommandOutput",
        Status::Cleared => "Cleared",
        other => {
            // The discriminant identifies the variant without its payload.
            let _ = other;
            "Status"
        }
    }
}

fn completion(id: u64, action: &str, elapsed: Duration, outcome: &str) -> String {
    format!(
        "COMPLETE id={id} action={action} outcome={outcome} processing_us={}",
        elapsed.as_micros()
    )
}

#[cfg(test)]
mod tests;
