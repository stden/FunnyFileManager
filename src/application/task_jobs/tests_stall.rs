//! Purpose: Tests for requests that do not come back: stale at exactly
//! 5 s, abandoned at exactly 10 s, no thread started beside an abandoned
//! one, a list reopened behind a stuck read, and a controller dropped
//! while a read never returns.

use std::time::Instant;

use super::tests::{answer, just_before, pump_until_readers, sampled, Fixture, WAIT};
use crate::application::Redraw;
use crate::models::{Health, SampleError, Schedule, Table, OVERDUE_AFTER, STALL_LIMIT, WARM_UP};
use crate::test_support::run_with_timeout;

/// The old session's request is still in flight: its thread is abandoned,
/// and the new session's starts once it has ended.
#[test]
fn a_new_session_waits_for_the_thread_still_reading_for_the_old_one() {
    let (mut fixture, host, gate) = Fixture::gated();
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    assert!(gate.wait_for_entries(1, WAIT));
    fixture.open();
    assert_eq!(fixture.pump(t0), Redraw::Unchanged);
    assert_eq!(
        host.readers_opened(),
        1,
        "no thread beside the abandoned one"
    );
    assert!(matches!(fixture.table(), Table::Waiting));
    gate.open(answer());
    pump_until_readers(&mut fixture, &host, 2, t0);
    assert!(gate.wait_for_entries(2, WAIT), "the new session asked");
    assert!(
        matches!(fixture.table(), Table::Waiting),
        "the old answer is dropped"
    );
}

#[test]
fn a_request_is_stale_at_exactly_five_seconds() {
    let t0 = Instant::now();
    let (mut fixture, _host, gate) = sampled(t0);
    let t1 = t0 + WARM_UP;
    fixture.pump(t1);
    assert!(gate.wait_for_entries(2, WAIT));
    assert_eq!(
        fixture.pump(t1 + just_before(OVERDUE_AFTER)),
        Redraw::Unchanged
    );
    assert_eq!(fixture.list().health(), Some(Health::Fresh));
    assert_eq!(fixture.pump(t1 + OVERDUE_AFTER), Redraw::Needed);
    assert_eq!(fixture.list().health(), Some(Health::Overdue));
    assert_eq!(
        fixture.pump(t1 + OVERDUE_AFTER),
        Redraw::Unchanged,
        "marked once"
    );
}

#[test]
fn a_request_is_abandoned_at_exactly_ten_seconds_and_asked_again_by_a_new_thread() {
    let t0 = Instant::now();
    let (mut fixture, host, gate) = sampled(t0);
    let t1 = t0 + WARM_UP;
    fixture.pump(t1);
    assert!(gate.wait_for_entries(2, WAIT));
    fixture.pump(t1 + just_before(STALL_LIMIT));
    assert_eq!(fixture.list().health(), Some(Health::Overdue));
    assert_eq!(fixture.pump(t1 + STALL_LIMIT), Redraw::Needed);
    assert_eq!(fixture.list().health(), Some(Health::Stalled));
    assert_eq!(fixture.list().refresh().schedule(), Schedule::Due);
    let later = t1 + STALL_LIMIT + STALL_LIMIT;
    assert_eq!(fixture.pump(later), Redraw::Unchanged);
    assert_eq!(
        host.readers_opened(),
        1,
        "no thread beside the abandoned one"
    );
    gate.open(answer());
    pump_until_readers(&mut fixture, &host, 2, later);
    assert!(gate.wait_for_entries(3, WAIT), "the new thread asked again");
    assert_eq!(
        fixture.list().health(),
        Some(Health::Stalled),
        "the late answer is dropped"
    );
}

#[test]
fn a_first_sample_that_stalls_says_the_reader_is_not_responding() {
    let (mut fixture, _host, gate) = Fixture::gated();
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    assert!(gate.wait_for_entries(1, WAIT));
    assert_eq!(fixture.pump(t0 + STALL_LIMIT), Redraw::Needed);
    assert!(matches!(
        fixture.table(),
        Table::Failed(SampleError::WorkerStalled)
    ));
    let session = fixture.list().session();
    let stalled = SampleError::WorkerStalled;
    assert!(
        !fixture.jobs.logged.first(session, stalled),
        "already logged"
    );
}

/// Reopened while the last session's read is still stuck, the list cannot
/// start a thread; once that read has gone unanswered for the stall limit,
/// it says so instead of looking as if it were starting.
#[test]
fn a_list_reopened_behind_a_stuck_read_says_the_reader_is_not_responding() {
    let (mut fixture, host, gate) = Fixture::gated();
    fixture.open();
    let t0 = Instant::now();
    fixture.pump(t0);
    assert!(gate.wait_for_entries(1, WAIT));
    fixture.open();
    assert_eq!(
        fixture.pump(t0 + just_before(STALL_LIMIT)),
        Redraw::Unchanged
    );
    assert!(matches!(fixture.table(), Table::Waiting));
    assert_eq!(fixture.pump(t0 + STALL_LIMIT), Redraw::Needed);
    let stalled = || matches!(fixture.table(), Table::Failed(SampleError::WorkerStalled));
    assert!(stalled());
    assert_eq!(
        fixture.pump(t0 + STALL_LIMIT),
        Redraw::Unchanged,
        "said once"
    );
    gate.open(answer());
    pump_until_readers(&mut fixture, &host, 2, t0 + STALL_LIMIT);
    assert!(
        gate.wait_for_entries(2, WAIT),
        "the list asks once a thread starts"
    );
    gate.open(answer());
    fixture.pump_until_needed(t0 + STALL_LIMIT);
    assert!(fixture.list().snapshot().is_some());
}

#[test]
fn dropping_the_controller_never_waits_on_a_read_that_never_returns() {
    let (mut fixture, _host, gate) = Fixture::gated();
    fixture.open();
    fixture.pump(Instant::now());
    assert!(gate.wait_for_entries(1, WAIT));
    let jobs = std::mem::take(&mut fixture.jobs);
    assert_eq!(run_with_timeout(WAIT, move || drop(jobs)), Some(()));
    drop(gate);
}
