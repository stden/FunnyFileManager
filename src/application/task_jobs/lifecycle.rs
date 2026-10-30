//! Purpose: a sampling worker's life — started for a list, retired when the list closes, and lost for good with its error logged once per session.

use std::io;
use std::mem;

use super::worker::{spawn, Flight, Worker};
use super::{Sampler, TaskJobs};
use crate::application::Redraw;
use crate::diagnostics::log_error;
use crate::models::{SampleError, Schedule, TaskHostSlot, TaskList, TaskSession};

impl TaskJobs {
    /// The session the running worker serves.
    pub(super) fn session(&self) -> Option<TaskSession> {
        match &self.sampler {
            Sampler::Running(worker) => Some(worker.session),
            Sampler::Idle | Sampler::Abandoned { .. } => None,
        }
    }

    /// The list is closed or reopened: an idle worker is dropped, and its
    /// thread ends at its next receive; one with a request in flight is
    /// abandoned.
    pub(super) fn retire(&mut self) {
        self.prober.retire();
        self.sampler = match mem::take(&mut self.sampler) {
            Sampler::Running(Worker {
                handle,
                flight: Flight::Since(since),
                ..
            }) => Sampler::Abandoned { handle, since },
            Sampler::Running(Worker {
                flight: Flight::Idle,
                ..
            }) => Sampler::Idle,
            kept @ (Sampler::Idle | Sampler::Abandoned { .. }) => kept,
        };
    }

    /// Starts a worker for `list` when it has none and may have one: not
    /// while an abandoned thread still runs, and never again once this
    /// session's worker is gone.
    pub(super) fn start(&mut self, list: &mut TaskList, host: &TaskHostSlot) -> Redraw {
        match &self.sampler {
            Sampler::Running(_) => return Redraw::Unchanged,
            Sampler::Abandoned { handle, .. } if !handle.is_finished() => {
                return Redraw::Unchanged;
            }
            Sampler::Idle | Sampler::Abandoned { .. } => {}
        }
        if list.refresh().schedule() == Schedule::Stopped {
            return Redraw::Unchanged;
        }
        self.sampler = Sampler::Idle;
        let reader = host
            .host()
            .map_or(Err(SampleError::Unsupported), |host| host.reader());
        match reader.map(|reader| spawn(reader, list.session())) {
            Ok(Ok(worker)) => {
                self.sampler = Sampler::Running(worker);
                Redraw::Unchanged
            }
            Ok(Err(error)) => self.unavailable(list, &error),
            Err(error) => self.lose(list, error),
        }
    }

    /// The thread could not be started: said once, and not tried again this
    /// session.
    pub(super) fn unavailable(&mut self, list: &mut TaskList, error: &io::Error) -> Redraw {
        log_error(&format!(
            "Task list: cannot start the process reader: {error}"
        ));
        self.lose(list, SampleError::WorkerUnavailable)
    }

    /// The session's worker is gone for good: the list shows `error` and
    /// asks for nothing more.
    pub(super) fn lose(&mut self, list: &mut TaskList, error: SampleError) -> Redraw {
        self.log_once(list.session(), error);
        list.worker_lost(error);
        Redraw::Needed
    }

    /// Logs `error` the first time `session` meets it: a table that fails
    /// on every refresh writes one line, not one per second and a half.
    pub(super) fn log_once(&mut self, session: TaskSession, error: SampleError) {
        if self.logged.first(session, error) {
            log_error(&format!("Task list: {error}"));
        }
    }
}
