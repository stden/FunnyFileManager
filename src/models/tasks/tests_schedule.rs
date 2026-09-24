//! Purpose: Tests for the refresh state machine: every transition of the
//! design's table, with synthetic instants and the boundaries to the
//! millisecond.

use std::time::{Duration, Instant};

use super::{
    Arrival, Cadence, Refresh, Request, Schedule, OVERDUE_AFTER, REFRESH_INTERVAL, STALL_LIMIT,
    WARM_UP,
};

const MS: Duration = Duration::from_millis(1);

/// A refresh that sampled once at `start` and is live again.
fn after_first_result(start: Instant) -> Refresh {
    let mut refresh = Refresh::default();
    assert_eq!(refresh.take_request(start), Some(Request::Explicit));
    assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Apply);
    refresh
}

#[test]
fn the_timings_are_the_designs() {
    assert_eq!(REFRESH_INTERVAL, Duration::from_millis(1_500));
    assert_eq!(WARM_UP, Duration::from_millis(500));
    assert_eq!(OVERDUE_AFTER, Duration::from_secs(5));
    assert_eq!(STALL_LIMIT, Duration::from_secs(10));
}

#[test]
fn an_opened_list_asks_at_once_and_never_twice_in_flight() {
    let start = Instant::now();
    let mut refresh = Refresh::default();
    assert_eq!(refresh.cadence(), Cadence::Live);
    assert_eq!(refresh.schedule(), Schedule::Due);
    assert_eq!(refresh.take_request(start), Some(Request::Explicit));
    assert_eq!(refresh.in_flight_since(), Some(start));
    assert_eq!(refresh.take_request(start + STALL_LIMIT), None);
}

#[test]
fn the_next_sample_is_due_exactly_one_interval_after_the_last_result() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    assert_eq!(refresh.schedule(), Schedule::At(start + REFRESH_INTERVAL));
    assert_eq!(refresh.in_flight_since(), None);
    assert_eq!(
        refresh.take_request(start + REFRESH_INTERVAL.saturating_sub(MS)),
        None
    );
    let due = start + REFRESH_INTERVAL;
    assert_eq!(refresh.take_request(due), Some(Request::Automatic));
    assert_eq!(
        refresh.schedule(),
        Schedule::Waiting {
            since: due,
            request: Request::Automatic
        }
    );
}

#[test]
fn holding_discards_an_automatic_result_so_the_screen_really_holds() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    let due = start + REFRESH_INTERVAL;
    refresh.take_request(due);
    refresh.toggle_hold();
    assert_eq!(refresh.cadence(), Cadence::Held);
    assert_eq!(
        refresh.in_flight_since(),
        Some(due),
        "the request in flight stays"
    );
    assert_eq!(
        refresh.completed(due + MS, REFRESH_INTERVAL),
        Arrival::Discard
    );
    assert_eq!(refresh.schedule(), Schedule::Held);
    assert_eq!(refresh.take_request(due + STALL_LIMIT), None);
}

#[test]
fn holding_keeps_a_sample_already_asked_for() {
    let start = Instant::now();
    let mut opened = Refresh::default();
    opened.toggle_hold();
    assert_eq!(
        opened.schedule(),
        Schedule::Due,
        "the list's first table is still wanted"
    );
    assert_eq!(opened.take_request(start), Some(Request::Explicit));
    assert_eq!(opened.completed(start, WARM_UP), Arrival::Apply);
    assert_eq!(opened.schedule(), Schedule::Held);

    let mut refreshed = after_first_result(start);
    refreshed.refresh_now();
    refreshed.toggle_hold();
    assert_eq!(refreshed.take_request(start), Some(Request::Explicit));
}

#[test]
fn holding_between_samples_stops_the_schedule_and_resuming_samples_at_once() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    refresh.toggle_hold();
    assert_eq!(refresh.schedule(), Schedule::Held);
    refresh.toggle_hold();
    assert_eq!(
        (refresh.cadence(), refresh.schedule()),
        (Cadence::Live, Schedule::Due)
    );
}

#[test]
fn refreshing_while_held_samples_once_and_holds_again() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    refresh.toggle_hold();
    refresh.refresh_now();
    assert_eq!(refresh.schedule(), Schedule::Due);
    assert_eq!(refresh.take_request(start), Some(Request::Explicit));
    assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Apply);
    assert_eq!(refresh.schedule(), Schedule::Held);
}

#[test]
fn refreshing_with_a_sample_in_flight_asks_for_that_one_instead_of_another() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    let due = start + REFRESH_INTERVAL;
    refresh.take_request(due);
    refresh.refresh_now();
    assert_eq!(
        refresh.schedule(),
        Schedule::Waiting {
            since: due,
            request: Request::Explicit
        }
    );
    refresh.toggle_hold();
    assert_eq!(refresh.completed(due, REFRESH_INTERVAL), Arrival::Apply);
}

#[test]
fn a_stalled_request_makes_a_sample_due_again() {
    let start = Instant::now();
    let mut refresh = Refresh::default();
    refresh.take_request(start);
    refresh.stalled();
    assert_eq!(refresh.schedule(), Schedule::Due);

    let mut idle = after_first_result(start);
    idle.stalled();
    assert_eq!(idle.schedule(), Schedule::At(start + REFRESH_INTERVAL));
}

/// A timer sample that stalls while the screen is held is not asked again:
/// its answer would have been discarded, so asking again would change a
/// held screen. One the user asked for is asked again, held or not.
#[test]
fn a_stalled_timer_sample_leaves_a_held_screen_held() {
    let start = Instant::now();
    let due = start + REFRESH_INTERVAL;
    let mut automatic = after_first_result(start);
    automatic.take_request(due);
    automatic.toggle_hold();
    automatic.stalled();
    assert_eq!(automatic.schedule(), Schedule::Held);
    assert_eq!(automatic.take_request(due + STALL_LIMIT), None);

    let mut explicit = after_first_result(start);
    explicit.take_request(due);
    explicit.toggle_hold();
    explicit.refresh_now();
    explicit.stalled();
    assert_eq!(explicit.schedule(), Schedule::Due);
}

#[test]
fn a_stopped_refresh_asks_for_nothing_ever_again() {
    let start = Instant::now();
    let mut refresh = after_first_result(start);
    refresh.stop();
    refresh.refresh_now();
    refresh.toggle_hold();
    refresh.toggle_hold();
    assert_eq!(refresh.schedule(), Schedule::Stopped);
    assert_eq!(refresh.take_request(start + STALL_LIMIT), None);
    assert_eq!(refresh.completed(start, REFRESH_INTERVAL), Arrival::Discard);
}

#[test]
fn a_result_nobody_waited_for_counts_as_automatic() {
    let start = Instant::now();
    let mut live = after_first_result(start);
    assert_eq!(live.completed(start, WARM_UP), Arrival::Apply);
    assert_eq!(live.schedule(), Schedule::At(start + WARM_UP));

    live.toggle_hold();
    assert_eq!(live.completed(start, WARM_UP), Arrival::Discard);
}
