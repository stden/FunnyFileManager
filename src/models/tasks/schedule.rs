//! Purpose: When the task list samples next — live or held, a sample due, one
//! at a set time, one in flight, or none ever again — and how fresh the
//! table on screen is.
//!
//! The model only says what is due; the controller that owns the sampling
//! thread asks it on every loop pass, with the time passed in, so every
//! transition here is tested with synthetic instants. Applying a sample is
//! here too: whether a result is shown depends on the schedule it answers.

use std::time::{Duration, Instant};

use super::{Sample, SampleError, Snapshot, Table, TaskList};

/// Time between one sample completing and the next being taken: htop's
/// default of fifteen tenths.
pub const REFRESH_INTERVAL: Duration = Duration::from_millis(1_500);
/// The pause after a session's first snapshot, so the first CPU shares appear
/// quickly; at 100 clock ticks a second it still resolves 2 % per process.
pub const WARM_UP: Duration = Duration::from_millis(500);
/// A sample in flight this long leaves the table marked stale.
pub const OVERDUE_AFTER: Duration = Duration::from_secs(5);
/// A sample in flight this long is abandoned as not responding.
pub const STALL_LIMIT: Duration = Duration::from_secs(10);

/// Whether the table follows the system: `F11` and `Z` switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cadence {
    #[default]
    Live,
    Held,
}

/// Why a sample was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// The interval ran out.
    Automatic,
    /// The list opened, `Ctrl+R` was pressed, or a signal or renice finished:
    /// shown even while the table is held.
    Explicit,
}

/// What happens next.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Schedule {
    /// Sample as soon as the controller can: where a list that just opened
    /// starts.
    #[default]
    Due,
    /// Sample once this instant has come.
    At(Instant),
    /// A sample is in flight.
    Waiting { since: Instant, request: Request },
    /// Held: nothing until `F11`, `Z` or `Ctrl+R`.
    Held,
    /// The sampling worker is gone for this session; reopening the list
    /// starts a new one. Nothing is ever asked for again.
    Stopped,
}

/// Whether a result that arrived is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Apply,
    /// Held, and nobody asked for it: the screen really holds.
    Discard,
}

/// How fresh the table on screen is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Fresh,
    /// The sample in flight is late: ` · stale `.
    Overdue,
    /// The sample in flight was abandoned: ` · not responding `.
    Stalled,
    /// The last sample failed, and the table shows the one before it.
    Failed(SampleError),
}

/// The cadence and the schedule together: the refresh state machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Refresh {
    cadence: Cadence,
    schedule: Schedule,
}

impl Refresh {
    #[must_use]
    pub const fn cadence(self) -> Cadence {
        self.cadence
    }

    #[must_use]
    pub const fn schedule(self) -> Schedule {
        self.schedule
    }

    /// When the sample in flight was asked for, if one is.
    #[must_use]
    pub const fn in_flight_since(self) -> Option<Instant> {
        match self.schedule {
            Schedule::Waiting { since, .. } => Some(since),
            Schedule::Due | Schedule::At(_) | Schedule::Held | Schedule::Stopped => None,
        }
    }

    /// Asks for a sample when one is due at `now`, and says why; `None`
    /// when nothing is due.
    pub fn take_request(&mut self, now: Instant) -> Option<Request> {
        let request = match self.schedule {
            Schedule::Due => Request::Explicit,
            Schedule::At(at) if now >= at => Request::Automatic,
            Schedule::At(_) | Schedule::Waiting { .. } | Schedule::Held | Schedule::Stopped => {
                return None;
            }
        };
        self.schedule = Schedule::Waiting {
            since: now,
            request,
        };
        Some(request)
    }

    /// A result arrived at `now`: whether to show it, and the next sample
    /// `pause` after it when live. The interval runs from completion, so a
    /// slow sample stretches the period instead of queueing reads.
    pub fn completed(&mut self, now: Instant, pause: Duration) -> Arrival {
        let request = match self.schedule {
            Schedule::Waiting { request, .. } => request,
            Schedule::Stopped => return Arrival::Discard,
            Schedule::Due | Schedule::At(_) | Schedule::Held => Request::Automatic,
        };
        self.schedule = match self.cadence {
            Cadence::Live => now.checked_add(pause).map_or(Schedule::Due, Schedule::At),
            Cadence::Held => Schedule::Held,
        };
        match (self.cadence, request) {
            (Cadence::Held, Request::Automatic) => Arrival::Discard,
            _ => Arrival::Apply,
        }
    }

    /// `F11` or `Z`. Holding keeps a sample already in flight, whose result
    /// shows only if it was asked for, and one asked for but not yet taken
    /// (`Due`: the opening, `Ctrl+R`, a finished kill), which is shown even
    /// while held; only the timer stops. Resuming samples at once.
    pub fn toggle_hold(&mut self) {
        self.cadence = match self.cadence {
            Cadence::Live => Cadence::Held,
            Cadence::Held => Cadence::Live,
        };
        self.schedule = match (self.cadence, self.schedule) {
            (_, kept @ (Schedule::Due | Schedule::Waiting { .. } | Schedule::Stopped)) => kept,
            (Cadence::Held, Schedule::At(_) | Schedule::Held) => Schedule::Held,
            (Cadence::Live, Schedule::At(_) | Schedule::Held) => Schedule::Due,
        };
    }

    /// `Ctrl+R`, or a signal or renice that just finished: one sample now,
    /// shown even while held. A sample already in flight is not doubled: it
    /// becomes the one asked for.
    pub fn refresh_now(&mut self) {
        self.schedule = match self.schedule {
            Schedule::Waiting { since, .. } => Schedule::Waiting {
                since,
                request: Request::Explicit,
            },
            Schedule::Stopped => Schedule::Stopped,
            Schedule::Due | Schedule::At(_) | Schedule::Held => Schedule::Due,
        };
    }

    /// The sample in flight was abandoned: ask again once a worker can —
    /// unless the screen is held and nobody asked for it, whose answer would
    /// have been discarded anyway.
    pub fn stalled(&mut self) {
        if let Schedule::Waiting { request, .. } = self.schedule {
            self.schedule = match (self.cadence, request) {
                (Cadence::Held, Request::Automatic) => Schedule::Held,
                (Cadence::Live, _) | (Cadence::Held, Request::Explicit) => Schedule::Due,
            };
        }
    }

    /// The worker is gone: ask for nothing more this session.
    pub fn stop(&mut self) {
        self.schedule = Schedule::Stopped;
    }
}

impl TaskList {
    /// Asks for a sample when one is due at `now`; `None` when nothing is.
    pub fn take_request(&mut self, now: Instant) -> Option<Request> {
        self.refresh.take_request(now)
    }

    /// A sample arrived at `now`, or failed. Shown unless the table is held
    /// and nobody asked for it; the next is due after the warm-up following
    /// the session's first snapshot, then after the refresh interval.
    pub fn apply_sample(&mut self, result: Result<Sample, SampleError>, now: Instant) {
        let first = matches!(self.table, Table::Waiting | Table::Failed(_));
        let pause = match &result {
            Ok(_) if first => WARM_UP,
            Ok(_) | Err(_) => REFRESH_INTERVAL,
        };
        if self.refresh.completed(now, pause) == Arrival::Discard {
            self.clear_overdue();
            return;
        }
        match result {
            Ok(sample) => self.adopt(sample),
            Err(error) => self.record_failure(error),
        }
    }

    /// The sampling worker is gone for this session: the error is shown and
    /// nothing is asked for again.
    pub fn worker_lost(&mut self, error: SampleError) {
        self.record_failure(error);
        self.refresh.stop();
    }

    /// The sample in flight is late: a fresh table is marked stale. A worse
    /// mark already shown stays.
    pub fn mark_overdue(&mut self) {
        if let Table::Ready { health, .. } = &mut self.table {
            if *health == Health::Fresh {
                *health = Health::Overdue;
            }
        }
    }

    /// The late sample did arrive, though a hold kept it off the screen:
    /// nothing is in flight to be late any more.
    fn clear_overdue(&mut self) {
        if let Table::Ready { health, .. } = &mut self.table {
            if *health == Health::Overdue {
                *health = Health::Fresh;
            }
        }
    }

    /// The sample in flight was abandoned: the table is marked not
    /// responding and a sample is due again. Without a table yet, the reader
    /// that never answered is the error shown in its place, so a first
    /// sample stuck in the kernel does not look like one still starting.
    pub fn mark_stalled(&mut self) {
        match &mut self.table {
            Table::Ready { health, .. } => *health = Health::Stalled,
            Table::Waiting | Table::Failed(_) => {
                self.table = Table::Failed(SampleError::WorkerStalled);
            }
        }
        self.refresh.stalled();
    }

    /// `F11` and `Z`.
    pub fn toggle_hold(&mut self) {
        self.refresh.toggle_hold();
    }

    /// `Ctrl+R`, and after a signal or a renice: one sample now.
    pub fn refresh_now(&mut self) {
        self.refresh.refresh_now();
    }

    fn adopt(&mut self, sample: Sample) {
        let (snapshot, baselines) = Snapshot::derive(&self.baselines, sample);
        self.baselines = baselines;
        self.selection.prune(snapshot.rows());
        self.table = Table::Ready {
            snapshot: Box::new(snapshot),
            health: Health::Fresh,
        };
        self.rebuild();
    }

    fn record_failure(&mut self, error: SampleError) {
        match &mut self.table {
            Table::Ready { health, .. } => *health = Health::Failed(error),
            Table::Waiting | Table::Failed(_) => self.table = Table::Failed(error),
        }
    }
}
