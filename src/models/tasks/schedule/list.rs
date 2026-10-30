//! Purpose: Apply the refresh schedule to task samples and table health;
//! the parent schedule model decides whether each arriving sample is current.

use std::time::Instant;

use super::super::{Sample, SampleError, Snapshot, Table, TaskList};
use super::{Arrival, Health, Request, REFRESH_INTERVAL, WARM_UP};

impl TaskList {
    /// Asks for a sample when one is due at `now`; `None` when nothing is.
    pub fn take_request(&mut self, now: Instant) -> Option<Request> {
        self.refresh.take_request(now)
    }

    /// A sample arrived at `now`, or failed. Shown unless held without an
    /// explicit request or superseded by a control; the next is due after
    /// the first snapshot's warm-up, then after the refresh interval.
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

    /// A discarded sample arrived: nothing is in flight to be late any more.
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

    /// `Ctrl+R`: request one sample, accepting a usable read already in flight.
    pub fn refresh_now(&mut self) {
        self.refresh.refresh_now();
        self.network.diagnostics.request_now();
    }

    /// A process control finished: request a sample begun after its effects.
    pub(crate) fn refresh_after_control(&mut self) {
        self.refresh.refresh_after_control();
    }

    fn adopt(&mut self, sample: Sample) {
        let gateway = sample.network.gateway;
        let dns_servers = sample.network.dns_servers.clone();
        let (snapshot, baselines) = Snapshot::derive(&self.baselines, sample);
        self.baselines = baselines;
        self.selection.prune(snapshot.rows());
        self.table = Table::Ready {
            snapshot: Box::new(snapshot),
            health: Health::Fresh,
        };
        self.network
            .diagnostics
            .sync_endpoints(gateway, &dns_servers);
        self.clamp_net_cursor();
        self.rebuild();
    }

    fn record_failure(&mut self, error: SampleError) {
        match &mut self.table {
            Table::Ready { health, .. } => *health = Health::Failed(error),
            Table::Waiting | Table::Failed(_) => self.table = Table::Failed(error),
        }
    }
}
