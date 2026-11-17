//! Purpose: unit tests for the design's timings, the first request, and the
//! interval from one result to the next sample.

use super::*;

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
