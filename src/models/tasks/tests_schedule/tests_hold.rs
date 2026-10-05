//! Purpose: unit tests for holding the screen and refreshing by hand.

use super::*;

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
