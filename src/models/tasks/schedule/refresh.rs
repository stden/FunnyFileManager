//! Purpose: The refresh state machine — the cadence and the schedule together
//! — and every transition the keys, the timer and the sampling worker make.

use std::time::{Duration, Instant};

use super::{Arrival, Cadence, Request, Schedule};

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
