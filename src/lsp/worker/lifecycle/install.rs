//! Purpose: Keep installation responsive to document edits, close, and application shutdown.

use super::{report, DocumentId, Envelope, Event, Host, ServerState, Session, Start, SyncSender};
use crate::lsp::install::{InstallError, Job};

pub(super) struct Preparation {
    pub start: Start,
    job: Job,
}

impl Host {
    pub(super) fn install(&mut self, start: Start, events: &SyncSender<Envelope>) {
        let owner = start.snapshot.stamp.id;
        let job = self
            .cache
            .as_ref()
            .ok_or(InstallError::Missing(
                "LSP installation requires a user configuration directory",
            ))
            .and_then(|cache| {
                Job::start(
                    start.settings.clone(),
                    cache.clone(),
                    start.workspace.clone(),
                )
            });
        match job {
            Ok(job) => {
                report(events, owner, Event::State(ServerState::Installing));
                self.session = Session::Preparing(Box::new(Preparation { start, job }));
            }
            Err(error) => installation_failed(events, owner, error),
        }
    }

    pub(super) fn installation(&mut self, events: &SyncSender<Envelope>) {
        let Session::Preparing(preparing) = &self.session else {
            return;
        };
        let Some(result) = preparing.job.poll() else {
            return;
        };
        let Session::Preparing(preparing) = std::mem::take(&mut self.session) else {
            return;
        };
        let mut start = preparing.start;
        match result {
            Ok(settings) => {
                start.settings = settings;
                self.start(start, events);
            }
            Err(error) => installation_failed(events, start.snapshot.stamp.id, error),
        }
    }
}

fn installation_failed(events: &SyncSender<Envelope>, owner: DocumentId, error: InstallError) {
    crate::diagnostics::log_error(&format!("LSP installation: {error:?}"));
    report(events, owner, Event::InstallationFailed(error.label()));
}

#[cfg(test)]
mod tests;
