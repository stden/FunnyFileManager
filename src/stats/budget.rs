//! Purpose: How much one measurement may spend — the entries it may visit and
//! the wall-clock time it may take — and which of the two ran out first.

use std::time::{Duration, Instant};

/// The limit a measurement ran into. Named, so the message can say which one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Entries,
    Time,
}

/// What one measurement may spend, shared by every walk inside it.
#[derive(Debug, Clone)]
pub struct Budget {
    entries: usize,
    deadline: Option<Instant>,
    stop: Option<Stop>,
}

impl Budget {
    /// A budget of `entries`, and of `time` as well when one is given.
    #[must_use]
    pub fn new(entries: usize, time: Option<Duration>) -> Self {
        Self {
            entries,
            deadline: time.map(|time| Instant::now() + time),
            stop: None,
        }
    }

    /// The limit that ran out, if one did.
    #[must_use]
    pub const fn stop(&self) -> Option<Stop> {
        self.stop
    }

    /// Spend one entry. `false` once the budget is gone, and it stays gone.
    pub(super) fn spend(&mut self) -> bool {
        if self.stop.is_some() {
            return false;
        }
        if self.entries == 0 {
            self.stop = Some(Stop::Entries);
            return false;
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.stop = Some(Stop::Time);
            return false;
        }
        self.entries -= 1;
        true
    }
}
