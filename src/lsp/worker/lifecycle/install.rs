//! Purpose: Keep installation responsive to document edits, close, and application shutdown.

use super::{
    failed, report, Connection, DocumentId, Envelope, Event, Host, ServerSettings, ServerState,
    Session, Start, SyncSender,
};
use crate::lsp::install::{InstallError, Job};
use crate::lsp::worker::PumpError;

const MAX_IDLE_SERVERS: usize = 4;

pub(super) struct Preparation {
    pub start: Start,
    job: Job,
}

impl Host {
    pub(super) fn park(&mut self) {
        if let Session::Running(mut active) = std::mem::take(&mut self.session) {
            active.close_document();
            if self.idle.len() >= MAX_IDLE_SERVERS {
                self.idle.remove(0);
            }
            self.idle.push(*active);
        } else {
            self.session = Session::Closed;
        }
    }

    pub(super) fn restart(&mut self) {
        self.missing.clear();
        if let Session::Running(active) = std::mem::take(&mut self.session) {
            self.idle.retain(|item| {
                !(item.language == active.language
                    && item.root == active.root
                    && item.configured == active.configured)
            });
        } else {
            self.idle.clear();
        }
    }

    pub(super) fn cached_missing(
        &self,
        language: &str,
        configured: &ServerSettings,
    ) -> Option<&'static str> {
        self.missing
            .iter()
            .find(|(lang, cfg, _)| lang == language && cfg == configured)
            .map(|(_, _, message)| *message)
    }

    fn remember_missing(
        &mut self,
        language: &str,
        configured: &ServerSettings,
        message: &'static str,
    ) {
        if self.cached_missing(language, configured).is_none() {
            self.missing
                .push((language.to_owned(), configured.clone(), message));
        }
    }

    pub(super) fn take_idle(&mut self, start: &Start) -> Option<Connection> {
        let index = self.idle.iter().position(|item| {
            item.language == start.language
                && item.root == start.root
                && item.configured == start.configured
        })?;
        Some(self.idle.swap_remove(index))
    }

    pub(super) fn launch(&mut self, start: Start, events: &SyncSender<Envelope>) {
        let owner = start.snapshot.stamp.id;
        report(events, owner, Event::State(ServerState::Starting));
        match Connection::start(
            start.language.clone(),
            &start.settings,
            start.snapshot.clone(),
            &start.workspace,
        ) {
            Ok(mut connection) => {
                connection.root = start.root;
                connection.configured = start.configured;
                self.session = Session::Running(Box::new(connection));
                report(events, owner, Event::State(ServerState::Initializing));
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && start.settings.install.is_some() =>
            {
                self.install(start, events);
            }
            Err(error) => failed(events, owner, &error),
        }
    }

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
            Err(error) => self.fail_install(&start, owner, error, events),
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
            Err(error) => self.fail_install(&start, start.snapshot.stamp.id, error, events),
        }
    }

    fn fail_install(
        &mut self,
        start: &Start,
        owner: DocumentId,
        error: InstallError,
        events: &SyncSender<Envelope>,
    ) {
        if let InstallError::Missing(message) = error {
            self.remember_missing(&start.language, &start.configured, message);
        }
        installation_failed(events, owner, error);
    }

    pub(super) fn fail_running(
        &mut self,
        active: &Connection,
        failure: PumpError,
        events: &SyncSender<Envelope>,
    ) {
        match failure {
            PumpError::Disconnected(message) => {
                crate::diagnostics::log_error(message);
                report(events, active.owner, Event::State(ServerState::Failed));
            }
            PumpError::MissingComponent(hint) => {
                crate::diagnostics::log_error(hint);
                self.remember_missing(&active.language, &active.configured, hint);
                report(events, active.owner, Event::InstallationFailed(hint));
            }
        }
    }
}

fn installation_failed(events: &SyncSender<Envelope>, owner: DocumentId, error: InstallError) {
    crate::diagnostics::log_error(&format!("LSP installation: {error:?}"));
    report(events, owner, Event::InstallationFailed(error.label()));
}

#[cfg(test)]
mod tests;
