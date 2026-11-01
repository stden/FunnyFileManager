//! Purpose: The task list's sampling worker: one thread per opening, asked
//! for one sample at a time, watched for lateness, and never joined — plus
//! the companion non-blocking network probe runner.
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
//! what is due and how a result changes the table, this module only moves
//! requests and results between the two.

mod lateness;
mod lifecycle;
mod logged;
mod net_probe;
mod worker;

use std::mem;
use std::sync::mpsc::{TryRecvError, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::Redraw;
use crate::models::{App, Overlay, SampleError, TaskList};
use crate::performance::Performance;
use logged::Logged;
use net_probe::NetProbeRunner;
use worker::{Flight, Worker};

/// The performance statistics row of a sample's round trip.
const SAMPLE_ACTION: &str = "Background/Tasks/Sample";

/// The controller `LoopState` owns: the sampling thread, the network prober,
/// and the errors already logged.
#[derive(Default)]
pub(super) struct TaskJobs {
    sampler: Sampler,
    prober: NetProbeRunner,
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
        let probed = self.prober.pump(list, task_host, performance, now);
        arrived.or(late).or(started).or(sent).or(probed)
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

    /// Sends the request the list says is due, if the worker can take one.
    fn request(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
        let Sampler::Running(worker) = &mut self.sampler else {
            return Redraw::Unchanged;
        };
        if list.take_request(now).is_none() {
            return Redraw::Unchanged;
        }
        match worker.requests.try_send(()) {
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
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_failures;
#[cfg(test)]
mod tests_net_probe;
#[cfg(test)]
mod tests_spawn;
#[cfg(test)]
mod tests_stall;
