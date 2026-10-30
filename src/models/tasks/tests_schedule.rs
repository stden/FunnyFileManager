//! Purpose: Tests for the refresh state machine: every transition of the
//! design's table, with synthetic instants and the boundaries to the
//! millisecond.

use std::time::{Duration, Instant};

use super::tests_support::{listing, process, shown};
use super::{
    Arrival, Cadence, Health, Refresh, Request, SampleError, Schedule, OVERDUE_AFTER,
    REFRESH_INTERVAL, STALL_LIMIT, WARM_UP,
};

mod tests_hold;
mod tests_interval;
mod tests_requests;

const MS: Duration = Duration::from_millis(1);

/// A refresh that sampled once at `start` and is live again.
fn after_first_result(start: Instant) -> Refresh {
    let mut refresh = Refresh::default();
    assert_eq!(refresh.take_request(start), Some(Request::Explicit));
    assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Apply);
    refresh
}
