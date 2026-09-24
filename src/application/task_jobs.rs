//! Purpose: The task list's sampling worker: one thread per opening, asked
//! for one sample at a time, watched for lateness, and never joined.
//!
//! A `/proc/<pid>/cmdline` read can block behind its process's mmap lock,
//! so samples are read off the UI thread. Every loop pass asks the model
//! what is due and costs a `match`, a `try_recv` and an `Instant` compare,
//! never I/O. A request unanswered at `OVERDUE_AFTER` marks the table stale;
//! at `STALL_LIMIT` its thread is abandoned and left to end on its own, so
//! neither closing the list nor quitting FFM ever waits on a read stuck in
//! the kernel. At most one thread samples, plus at most one abandoned: none
//! starts while an abandoned one is still running.
//!
//! `App` and `Overlay` stay free of threads and channels; the model decides
//! what is due and how a result changes the table, this file only moves
//! requests and results between the two.
//!
//! TODO(DRY): third worker — generalise with `WslJobs`. This is the second
//! request/response worker, so under the Rule of Three it stays separate.

mod logged;
mod worker;

use std::io;
use std::mem;
use std::sync::mpsc::{TryRecvError, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::Redraw;
use crate::diagnostics::log_error;
use crate::models::{
    App, Overlay, SampleError, Schedule, Table, TaskHostSlot, TaskList, TaskSession, OVERDUE_AFTER,
    STALL_LIMIT,
};
use crate::performance::Performance;
use logged::Logged;
use worker::{spawn, Flight, Worker};

/// The performance statistics row of a sample's round trip.
const SAMPLE_ACTION: &str = "Background/Tasks/Sample";

/// The controller `LoopState` owns: the sampling thread, if any, and the
/// errors already logged.
#[derive(Default)]
pub(super) struct TaskJobs {
    sampler: Sampler,
    logged: Logged,
}

/// The thread that samples for the open task list.
#[derive(Default)]
enum Sampler {
    /// No thread.
    #[default]
    Idle,
    Running(Worker),
    /// A thread whose request went unanswered, left to end on its own when
    /// its read returns. `since` is when that request was sent.
    Abandoned {
        handle: JoinHandle<()>,
        since: Instant,
    },
}

impl Sampler {
    /// The running thread left to end on its own, its request sent at
    /// `since`; nothing else changes.
    fn abandon(self, since: Instant) -> Self {
        match self {
            Self::Running(worker) => Self::Abandoned {
                handle: worker.handle,
                since,
            },
            kept @ (Self::Idle | Self::Abandoned { .. }) => kept,
        }
    }
}

impl TaskJobs {
    /// One loop pass at the current time.
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        self.pump_at(app, Instant::now())
    }

    /// One loop pass at `now`: retire a worker the list no longer needs,
    /// apply what arrived, watch what is late, start a worker and send what
    /// is due. `Needed` only when the list changed.
    fn pump_at(&mut self, app: &mut App, now: Instant) -> Redraw {
        let App {
            overlay,
            task_host,
            performance,
            ..
        } = app;
        let Overlay::Tasks(list) = overlay else {
            self.retire();
            return Redraw::Unchanged;
        };
        if self
            .session()
            .is_some_and(|session| session != list.session())
        {
            self.retire();
        }
        let arrived = self.drain(list, performance, now);
        let late = self.watch(list, now);
        let started = self.start(list, task_host);
        let sent = self.request(list, now);
        arrived.or(late).or(started).or(sent)
    }

    /// The session the running worker serves.
    fn session(&self) -> Option<TaskSession> {
        match &self.sampler {
            Sampler::Running(worker) => Some(worker.session),
            Sampler::Idle | Sampler::Abandoned { .. } => None,
        }
    }

    /// The list is closed or reopened: an idle worker is dropped, and its
    /// thread ends at its next receive; one with a request in flight is
    /// abandoned.
    fn retire(&mut self) {
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

    /// Applies the answer that arrived, if one did. A worker that is gone
    /// is not started again this session.
    fn drain(
        &mut self,
        list: &mut TaskList,
        performance: &mut Performance,
        now: Instant,
    ) -> Redraw {
        let Sampler::Running(worker) = &mut self.sampler else {
            return Redraw::Unchanged;
        };
        let answer = match worker.results.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return Redraw::Unchanged,
            Err(TryRecvError::Disconnected) => {
                self.sampler = Sampler::Idle;
                return self.lose(list, SampleError::WorkerStopped);
            }
        };
        if let Flight::Since(sent) = mem::replace(&mut worker.flight, Flight::Idle) {
            let round_trip = now.saturating_duration_since(sent);
            performance.record(SAMPLE_ACTION, round_trip, Duration::ZERO);
        }
        if let Err(error) = &answer {
            self.log_once(list.session(), *error);
        }
        list.apply_sample(answer, now);
        Redraw::Needed
    }

    /// Watches what the list is waiting for: the running worker's request,
    /// or, behind an abandoned thread, its first sample.
    fn watch(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
        match &self.sampler {
            Sampler::Running(_) => self.watch_request(list, now),
            Sampler::Abandoned { handle, since } if !handle.is_finished() => {
                let since = *since;
                self.stuck_behind(list, since, now)
            }
            Sampler::Idle | Sampler::Abandoned { .. } => Redraw::Unchanged,
        }
    }

    /// Marks a late request stale, and abandons the thread of one that
    /// stalled; the list asks again once a thread may start.
    fn watch_request(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
        let Some(since) = list.refresh().in_flight_since() else {
            return Redraw::Unchanged;
        };
        let late = now.saturating_duration_since(since);
        if late >= STALL_LIMIT {
            self.sampler = mem::take(&mut self.sampler).abandon(since);
            return self.stall(list);
        }
        if late < OVERDUE_AFTER {
            return Redraw::Unchanged;
        }
        let before = list.health();
        list.mark_overdue();
        if list.health() == before {
            Redraw::Unchanged
        } else {
            Redraw::Needed
        }
    }

    /// Starts a worker for `list` when it has none and may have one: not
    /// while an abandoned thread still runs, and never again once this
    /// session's worker is gone.
    fn start(&mut self, list: &mut TaskList, host: &TaskHostSlot) -> Redraw {
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

    /// A list still waiting for its first sample, while the thread
    /// abandoned at `since` has not answered for the stall limit: no thread
    /// can start for it until that one ends, so it says the reader is not
    /// responding rather than look as if it were starting.
    fn stuck_behind(&mut self, list: &mut TaskList, since: Instant, now: Instant) -> Redraw {
        let waiting = matches!(list.table(), Table::Waiting);
        if !waiting || now.saturating_duration_since(since) < STALL_LIMIT {
            return Redraw::Unchanged;
        }
        self.stall(list)
    }

    /// The reader has not answered for the stall limit: the list says so,
    /// and the log once.
    fn stall(&mut self, list: &mut TaskList) -> Redraw {
        self.log_once(list.session(), SampleError::WorkerStalled);
        list.mark_stalled();
        Redraw::Needed
    }

    /// The thread could not be started: said once, and not tried again this
    /// session.
    fn unavailable(&mut self, list: &mut TaskList, error: &io::Error) -> Redraw {
        log_error(&format!(
            "Task list: cannot start the process reader: {error}"
        ));
        self.lose(list, SampleError::WorkerUnavailable)
    }

    /// Sends the request the list says is due, if the worker can take one.
    fn request(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
        let Sampler::Running(worker) = &mut self.sampler else {
            return Redraw::Unchanged;
        };
        if list.take_request(now).is_none() {
            return Redraw::Unchanged;
        }
        match worker.requests.try_send(()) {
            // One request is in flight at a time, so the slot is free; were
            // it not, the request queued there is this one.
            Ok(()) | Err(TrySendError::Full(())) => {
                worker.flight = Flight::Since(now);
                Redraw::Unchanged
            }
            Err(TrySendError::Disconnected(())) => {
                self.sampler = Sampler::Idle;
                self.lose(list, SampleError::WorkerStopped)
            }
        }
    }

    /// The session's worker is gone for good: the list shows `error` and
    /// asks for nothing more.
    fn lose(&mut self, list: &mut TaskList, error: SampleError) -> Redraw {
        self.log_once(list.session(), error);
        list.worker_lost(error);
        Redraw::Needed
    }

    /// Logs `error` the first time `session` meets it: a table that fails
    /// on every refresh writes one line, not one per second and a half.
    fn log_once(&mut self, session: TaskSession, error: SampleError) {
        if self.logged.first(session, error) {
            log_error(&format!("Task list: {error}"));
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_failures;
#[cfg(test)]
mod tests_stall;
