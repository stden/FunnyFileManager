//! Purpose: When the task list samples next — live or held, a sample due, one
//! at a set time, one in flight, or none ever again — and how fresh the
//! table on screen is.
//!
//! The model only says what is due; the controller that owns the sampling
//! thread asks it on every loop pass, with the time passed in, so every
//! transition here is tested with synthetic instants. Applying a sample is
//! here too: whether a result is shown depends on the schedule it answers.

use std::time::{Duration, Instant};

use super::SampleError;

mod list;

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
    /// A control made the sample in flight obsolete: discard it, then ask
    /// for one that can observe the control's result, even while held.
    Obsolete { since: Instant },
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
    /// Held without an explicit request, or superseded by a control.
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
            Schedule::Waiting { since, .. } | Schedule::Obsolete { since } => Some(since),
            Schedule::Due | Schedule::At(_) | Schedule::Held | Schedule::Stopped => None,
        }
    }

    /// Asks for a sample when one is due at `now`, and says why; `None`
    /// when nothing is due.
    pub fn take_request(&mut self, now: Instant) -> Option<Request> {
        let request = match self.schedule {
            Schedule::Due => Request::Explicit,
            Schedule::At(at) if now >= at => Request::Automatic,
            Schedule::At(_)
            | Schedule::Waiting { .. }
            | Schedule::Obsolete { .. }
            | Schedule::Held
            | Schedule::Stopped => {
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
            Schedule::Obsolete { .. } => {
                self.schedule = Schedule::Due;
                return Arrival::Discard;
            }
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
            (
                _,
                kept @ (Schedule::Due
                | Schedule::Waiting { .. }
                | Schedule::Obsolete { .. }
                | Schedule::Stopped),
            ) => kept,
            (Cadence::Held, Schedule::At(_) | Schedule::Held) => Schedule::Held,
            (Cadence::Live, Schedule::At(_) | Schedule::Held) => Schedule::Due,
        };
    }

    /// `Ctrl+R`: one sample now, shown even while held. A usable sample in
    /// flight becomes the one asked for; an obsolete one still needs its
    /// replacement.
    pub fn refresh_now(&mut self) {
        self.schedule = match self.schedule {
            Schedule::Waiting { since, .. } => Schedule::Waiting {
                since,
                request: Request::Explicit,
            },
            kept @ (Schedule::Obsolete { .. } | Schedule::Stopped) => kept,
            Schedule::Due | Schedule::At(_) | Schedule::Held => Schedule::Due,
        };
    }

    /// A signal or renice finished: only a sample begun afterwards can
    /// reflect it. Keep an older read in flight until it finishes, without
    /// accepting its answer or starting another worker beside it.
    pub(crate) fn refresh_after_control(&mut self) {
        self.schedule = match self.schedule {
            Schedule::Waiting { since, .. } | Schedule::Obsolete { since } => {
                Schedule::Obsolete { since }
            }
            Schedule::Stopped => Schedule::Stopped,
            Schedule::Due | Schedule::At(_) | Schedule::Held => Schedule::Due,
        };
    }

    /// The sample in flight was abandoned: ask again once a worker can —
    /// unless the screen is held and nobody asked for it, whose answer would
    /// have been discarded anyway.
    pub fn stalled(&mut self) {
        self.schedule = match self.schedule {
            Schedule::Waiting { request, .. } => match (self.cadence, request) {
                (Cadence::Held, Request::Automatic) => Schedule::Held,
                (Cadence::Live, _) | (Cadence::Held, Request::Explicit) => Schedule::Due,
            },
            Schedule::Obsolete { .. } => Schedule::Due,
            kept @ (Schedule::Due | Schedule::At(_) | Schedule::Held | Schedule::Stopped) => kept,
        };
    }

    /// The worker is gone: ask for nothing more this session.
    pub fn stop(&mut self) {
        self.schedule = Schedule::Stopped;
    }
}
