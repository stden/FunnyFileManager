//! Purpose: watching the request in flight — stale at `OVERDUE_AFTER`, its thread abandoned at `STALL_LIMIT` — and a list stuck behind an abandoned thread.

use std::mem;
use std::time::Instant;

use super::{Sampler, TaskJobs};
use crate::application::Redraw;
use crate::models::{SampleError, Table, TaskList, OVERDUE_AFTER, STALL_LIMIT};

impl TaskJobs {
    /// Watches what the list is waiting for: the running worker's request,
    /// or, behind an abandoned thread, its first sample.
    pub(super) fn watch(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
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
    pub(super) fn watch_request(&mut self, list: &mut TaskList, now: Instant) -> Redraw {
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

    /// A list still waiting for its first sample, while the thread
    /// abandoned at `since` has not answered for the stall limit: no thread
    /// can start for it until that one ends, so it says the reader is not
    /// responding rather than look as if it were starting.
    pub(super) fn stuck_behind(
        &mut self,
        list: &mut TaskList,
        since: Instant,
        now: Instant,
    ) -> Redraw {
        let waiting = matches!(list.table(), Table::Waiting);
        if !waiting || now.saturating_duration_since(since) < STALL_LIMIT {
            return Redraw::Unchanged;
        }
        self.stall(list)
    }

    /// The reader has not answered for the stall limit: the list says so,
    /// and the log once.
    pub(super) fn stall(&mut self, list: &mut TaskList) -> Redraw {
        self.log_once(list.session(), SampleError::WorkerStalled);
        list.mark_stalled();
        Redraw::Needed
    }
}
